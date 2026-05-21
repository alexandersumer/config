use crate::commands::{generate_command, validate_command};
use crate::config_root::config_root_from_exe;
use crate::error::Result;
use crate::install::install_command;
use crate::registry::{get, load_yaml_file, string_key, validate_registry};
use crate::repair::repair_config_command;
use serde_yaml::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

type Mutation = fn(&Path) -> Result<&'static str>;

pub(crate) fn run_regression_tests() -> Result<()> {
    assert_clean_fixture_passes()?;
    let mutations: &[(&str, Mutation)] = &[
        ("missing skill file", mutate_missing_skill_file),
        (
            "front matter name mismatch",
            mutate_front_matter_name_mismatch,
        ),
        ("generated registry drift", mutate_generated_registry_drift),
        ("front matter only skill", mutate_front_matter_only_skill),
        ("thin skill body", mutate_thin_skill_body),
        ("placeholder skill body", mutate_placeholder_skill_body),
        ("unregistered skill file", mutate_unregistered_skill_file),
        ("broken prompt adapter", mutate_prompt_adapter_broken),
    ];
    for (name, mutate) in mutations {
        assert_mutation_fails(name, *mutate)?;
    }
    test_new_skill_generate_validate_flow()?;
    test_repair_config_command()?;
    test_install_command()?;
    test_link_safety()?;
    test_command_failures()?;
    test_git_fetch_ref_cleanup()?;
    test_terminal_title_config_and_format()?;
    Ok(())
}

fn test_repair_config_command() -> Result<()> {
    // Test the public behavior of repair-config: empty rovodev/prompts directory
    // should be replaced with a relative symlink to ../.agents/skills
    let fixture = copy_fixture()?;
    let prompts_link = fixture.path().join("rovodev/prompts");

    // Remove the existing symlink and create an empty directory instead
    fs::remove_file(&prompts_link).map_err(|err| {
        format!(
            "{}: cannot remove fixture symlink: {err}",
            prompts_link.display()
        )
    })?;
    fs::create_dir(&prompts_link).map_err(|err| {
        format!(
            "{}: cannot create empty directory: {err}",
            prompts_link.display()
        )
    })?;

    // Verify initial state: directory exists
    if !prompts_link.is_dir() {
        return Err("Setup failed: rovodev/prompts should be a directory".to_string());
    }

    // Run repair-config command
    repair_config_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
    ])?;

    // Verify the symlink now exists and points to the correct relative target
    if !prompts_link.is_symlink() {
        return Err(format!(
            "{}: should be a symlink after repair",
            prompts_link.display()
        ));
    }

    // Read the symlink and verify it's a relative path
    let link_target = fs::read_link(&prompts_link)
        .map_err(|err| format!("{}: cannot read symlink: {err}", prompts_link.display()))?;

    if link_target != PathBuf::from("../.agents/skills") {
        return Err(format!(
            "{}: symlink should point to ../.agents/skills; got {:?}",
            prompts_link.display(),
            link_target
        ));
    }

    // Verify the symlink resolves to the skills directory
    let resolved = prompts_link.canonicalize().map_err(|err| {
        format!(
            "{}: cannot canonicalize symlink: {err}",
            prompts_link.display()
        )
    })?;
    let expected = fixture
        .path()
        .join(".agents/skills")
        .canonicalize()
        .map_err(|err| {
            format!(
                "{}: cannot canonicalize skills directory: {err}",
                fixture.path().join(".agents/skills").display()
            )
        })?;

    if resolved != expected {
        return Err(format!(
            "{}: symlink resolves to {}; expected {}",
            prompts_link.display(),
            resolved.display(),
            expected.display()
        ));
    }

    Ok(())
}

fn test_new_skill_generate_validate_flow() -> Result<()> {
    let fixture = copy_fixture()?;
    let skill_dir = fixture.path().join(".agents/skills/real-e2e-fixture");
    fs::create_dir(&skill_dir).map_err(|err| {
        format!(
            "{}: cannot create fixture skill directory: {err}",
            skill_dir.display()
        )
    })?;
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: real-e2e-fixture\ndescription: Fixture skill proving generation discovers new valid skills through the public command path.\nregister_cmd: true\n---\n\nDrive the public command path, not a helper-only shortcut, when proving newly added skills are discoverable.\nGenerate the registry from the fixture checkout, then validate it exactly as the normal config command does.\nThe fixture body is intentionally concrete so failures prove registration behavior rather than placeholder rejection.\n",
    )
    .map_err(|err| format!("cannot write fixture skill: {err}"))?;

    let stale_errors = validate_registry(fixture.path()).join("\n");
    if !stale_errors.contains("generated content is not up to date") {
        return Err(format!(
            "new skill fixture should fail before registry generation; got:\n{stale_errors}"
        ));
    }

    generate_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
    ])?;
    generate_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
        "--check".to_string(),
    ])?;
    validate_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
    ])?;

    let generated = fs::read_to_string(fixture.path().join("rovodev/prompts.yml"))
        .map_err(|err| format!("cannot read generated fixture registry: {err}"))?;
    if !generated.contains("name: real-e2e-fixture")
        || !generated.contains("content_file: prompts/real-e2e-fixture/SKILL.md")
    {
        return Err("generated registry did not include new fixture skill".to_string());
    }
    Ok(())
}

pub(crate) fn test_install_command() -> Result<()> {
    let fixture = copy_fixture()?;
    let hidden_skill_dir = fixture.path().join(".agents/skills/.system");
    fs::create_dir_all(&hidden_skill_dir).map_err(|err| {
        format!(
            "{}: cannot create hidden fixture skill: {err}",
            hidden_skill_dir.display()
        )
    })?;
    fs::write(hidden_skill_dir.join("SKILL.md"), "hidden fixture\n")
        .map_err(|err| format!("cannot write hidden fixture skill: {err}"))?;
    let non_skill_dir = fixture.path().join(".agents/skills/not-a-skill");
    fs::create_dir(&non_skill_dir).map_err(|err| {
        format!(
            "{}: cannot create non-skill fixture directory: {err}",
            non_skill_dir.display()
        )
    })?;

    let home = tempfile::Builder::new()
        .prefix("tmp_rovodev_install_home_")
        .tempdir()
        .map_err(|err| format!("cannot create temp install home: {err}"))?;
    create_codex_system_skills(home.path())?;

    install_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])?;

    assert_symlink_resolves_to(
        &home.path().join(".agents"),
        &fixture.path().join(".agents"),
    )?;
    assert_symlink_resolves_to(
        &home.path().join(".rovodev/skills"),
        &fixture.path().join(".agents/skills"),
    )?;
    assert_symlink_resolves_to(
        &home.path().join(".rovodev/prompts"),
        &fixture.path().join(".agents/skills"),
    )?;
    assert_symlink_resolves_to(
        &home.path().join(".rovodev/prompts.yml"),
        &fixture.path().join("rovodev/prompts.yml"),
    )?;
    assert_symlink_resolves_to(
        &home.path().join(".codex/skills/apply-changes"),
        &fixture.path().join(".agents/skills/apply-changes"),
    )?;
    assert_symlink_resolves_to(
        &home.path().join(".codex/skills/describe-branch"),
        &fixture.path().join(".agents/skills/describe-branch"),
    )?;
    assert_symlink_resolves_to(
        &home.path().join(".zshrc"),
        &fixture.path().join("zsh/zshrc"),
    )?;
    assert_symlink_resolves_to(&home.path().join(".zsh"), &fixture.path().join("zsh"))?;
    assert_symlink_resolves_to(
        &home.path().join(".config/ghostty/config"),
        &fixture.path().join("ghostty/config"),
    )?;
    if home.path().join(".codex/skills/.system").is_symlink() {
        return Err("install replaced Codex-owned .system with a symlink".to_string());
    }
    if home.path().join(".codex/skills/.system/.system").exists() {
        return Err("install linked hidden .system as a custom Codex skill".to_string());
    }
    if home.path().join(".codex/skills/not-a-skill").exists() {
        return Err("install linked a directory without SKILL.md as a Codex skill".to_string());
    }

    install_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])?;
    Ok(())
}

pub(crate) fn test_link_safety() -> Result<()> {
    let missing_codex_fixture = copy_fixture()?;
    let missing_codex_home = tempfile::Builder::new()
        .prefix("tmp_codex_install_missing_system_")
        .tempdir()
        .map_err(|err| format!("cannot create missing Codex system home: {err}"))?;
    let missing_codex_error = install_command(&[
        "--config-root".to_string(),
        missing_codex_fixture.path().display().to_string(),
        "--home".to_string(),
        missing_codex_home.path().display().to_string(),
    ])
    .expect_err("install should require Codex .system skills before linking custom Codex skills");
    if !missing_codex_error.contains("Codex system skills directory") {
        return Err(format!(
            "unexpected missing Codex system error: {missing_codex_error}"
        ));
    }

    let codex_conflict_fixture = copy_fixture()?;
    let codex_conflict_home = tempfile::Builder::new()
        .prefix("tmp_codex_install_conflict_")
        .tempdir()
        .map_err(|err| format!("cannot create Codex conflict home: {err}"))?;
    create_codex_system_skills(codex_conflict_home.path())?;
    let conflicting_skill = codex_conflict_home
        .path()
        .join(".codex/skills/apply-changes");
    fs::create_dir(&conflicting_skill).map_err(|err| {
        format!(
            "{}: cannot create conflicting Codex skill: {err}",
            conflicting_skill.display()
        )
    })?;
    fs::write(conflicting_skill.join("keep"), "do not delete")
        .map_err(|err| format!("cannot write conflicting Codex skill file: {err}"))?;
    let codex_conflict_error = install_command(&[
        "--config-root".to_string(),
        codex_conflict_fixture.path().display().to_string(),
        "--home".to_string(),
        codex_conflict_home.path().display().to_string(),
    ])
    .expect_err("install should reject existing conflicting Codex skill entries");
    if !codex_conflict_error.contains("expected Codex skill symlink") {
        return Err(format!(
            "unexpected Codex conflict error: {codex_conflict_error}"
        ));
    }
    if !conflicting_skill.join("keep").is_file() {
        return Err("install removed data from conflicting Codex skill directory".to_string());
    }

    let repair_fixture = copy_fixture()?;
    let prompts_link = repair_fixture.path().join("rovodev/prompts");
    fs::remove_file(&prompts_link).map_err(|err| {
        format!(
            "{}: cannot remove fixture symlink: {err}",
            prompts_link.display()
        )
    })?;
    fs::create_dir(&prompts_link).map_err(|err| {
        format!(
            "{}: cannot create fixture directory: {err}",
            prompts_link.display()
        )
    })?;
    fs::write(prompts_link.join("keep"), "do not delete").map_err(|err| {
        format!(
            "{}: cannot write fixture file: {err}",
            prompts_link.display()
        )
    })?;
    let repair_error = repair_config_command(&[
        "--config-root".to_string(),
        repair_fixture.path().display().to_string(),
    ])
    .expect_err("repair-config should reject non-empty config prompt adapter directories");
    if !repair_error.contains("not an empty directory") {
        return Err(format!("unexpected repair-config error: {repair_error}"));
    }
    if !prompts_link.join("keep").is_file() {
        return Err(
            "repair-config removed data from non-empty config prompt adapter directory".to_string(),
        );
    }

    let install_fixture = copy_fixture()?;
    let home = tempfile::Builder::new()
        .prefix("tmp_rovodev_install_safety_")
        .tempdir()
        .map_err(|err| format!("cannot create temp install home: {err}"))?;
    create_codex_system_skills(home.path())?;
    fs::create_dir(home.path().join(".agents"))
        .map_err(|err| format!("cannot create fixture .agents directory: {err}"))?;
    fs::write(home.path().join(".agents/keep"), "do not delete")
        .map_err(|err| format!("cannot write fixture .agents file: {err}"))?;
    let install_error = install_command(&[
        "--config-root".to_string(),
        install_fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])
    .expect_err("install should reject non-empty home .agents directories");
    if !install_error.contains("not an empty directory") {
        return Err(format!("unexpected install error: {install_error}"));
    }
    if !home.path().join(".agents/keep").is_file() {
        return Err("install removed data from non-empty home .agents directory".to_string());
    }

    let matching_file_fixture = copy_fixture()?;
    let matching_file_home = tempfile::Builder::new()
        .prefix("tmp_config_install_matching_file_")
        .tempdir()
        .map_err(|err| format!("cannot create matching file home: {err}"))?;
    create_codex_system_skills(matching_file_home.path())?;
    fs::copy(
        matching_file_fixture.path().join("zsh/zshrc"),
        matching_file_home.path().join(".zshrc"),
    )
    .map_err(|err| format!("cannot copy matching zshrc fixture: {err}"))?;
    install_command(&[
        "--config-root".to_string(),
        matching_file_fixture.path().display().to_string(),
        "--home".to_string(),
        matching_file_home.path().display().to_string(),
    ])?;
    assert_symlink_resolves_to(
        &matching_file_home.path().join(".zshrc"),
        &matching_file_fixture.path().join("zsh/zshrc"),
    )?;

    let divergent_file_fixture = copy_fixture()?;
    let divergent_file_home = tempfile::Builder::new()
        .prefix("tmp_config_install_divergent_file_")
        .tempdir()
        .map_err(|err| format!("cannot create divergent file home: {err}"))?;
    create_codex_system_skills(divergent_file_home.path())?;
    fs::write(
        divergent_file_home.path().join(".zshrc"),
        "do not replace
",
    )
    .map_err(|err| format!("cannot write divergent zshrc fixture: {err}"))?;
    let divergent_file_error = install_command(&[
        "--config-root".to_string(),
        divergent_file_fixture.path().display().to_string(),
        "--home".to_string(),
        divergent_file_home.path().display().to_string(),
    ])
    .expect_err("install should reject divergent home config files");
    if !divergent_file_error.contains("different contents") {
        return Err(format!(
            "unexpected divergent file error: {divergent_file_error}"
        ));
    }
    if !divergent_file_home.path().join(".zshrc").is_file() {
        return Err("install removed divergent home config file".to_string());
    }
    Ok(())
}

pub(crate) fn test_command_failures() -> Result<()> {
    let drift_fixture = copy_fixture()?;
    let mut config = load_config(drift_fixture.path())?;
    let prompts = config
        .as_mapping_mut()
        .and_then(|mapping| mapping.get_mut(string_key("prompts")))
        .and_then(Value::as_sequence_mut)
        .ok_or_else(|| "fixture prompts.yml is missing prompts".to_string())?;
    let first_prompt = prompts
        .first_mut()
        .and_then(Value::as_mapping_mut)
        .ok_or_else(|| "fixture prompts.yml is missing prompt mapping".to_string())?;
    first_prompt.insert(
        string_key("description"),
        Value::String("drifted".to_string()),
    );
    write_config(drift_fixture.path(), &config)?;
    let generate_error = generate_command(&[
        "--config-root".to_string(),
        drift_fixture.path().display().to_string(),
        "--check".to_string(),
    ])
    .expect_err("generate --check should fail when prompts.yml has drifted");
    if !generate_error.contains("generated content is not up to date") {
        return Err(format!(
            "unexpected generate --check error: {generate_error}"
        ));
    }

    let invalid_fixture = copy_fixture()?;
    mutate_missing_skill_file(invalid_fixture.path())?;
    let validate_error = validate_command(&[
        "--config-root".to_string(),
        invalid_fixture.path().display().to_string(),
    ])
    .expect_err("validate should fail when validation errors exist");
    if !validate_error.contains("Skill validation failed") {
        return Err(format!("unexpected validate error: {validate_error}"));
    }
    Ok(())
}

fn test_git_fetch_ref_cleanup() -> Result<()> {
    let config_root = std::env::current_dir()
        .map_err(|err| format!("cannot determine config root for git cleanup test: {err}"))?;
    if !config_root.join("zsh/git-functions.zsh").is_file() {
        return Err(format!(
            "{}: git cleanup test must run from config repo root",
            config_root.display()
        ));
    }
    let work = tempfile::Builder::new()
        .prefix("tmp_git_fetch_ref_cleanup_")
        .tempdir()
        .map_err(|err| format!("cannot create temp git cleanup repo: {err}"))?;

    run_command(work.path(), "git", &["init", "--initial-branch=main"])?;
    run_command(
        work.path(),
        "git",
        &["config", "user.email", "test@example.com"],
    )?;
    run_command(work.path(), "git", &["config", "user.name", "Test User"])?;
    fs::write(work.path().join("README.md"), "fixture\n")
        .map_err(|err| format!("cannot write cleanup fixture README: {err}"))?;
    run_command(work.path(), "git", &["add", "README.md"])?;
    run_command(work.path(), "git", &["commit", "-m", "initial"])?;
    run_command(work.path(), "git", &["remote", "add", "origin", "."])?;

    run_command(
        work.path(),
        "git",
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    )?;
    run_command(
        work.path(),
        "git",
        &["update-ref", "refs/remotes/origin/NOISSUE/deleted", "HEAD"],
    )?;
    run_command(
        work.path(),
        "git",
        &["update-ref", "refs/remotes/origin/noIssue/locked", "HEAD"],
    )?;
    fs::create_dir_all(work.path().join(".git/refs/remotes/origin/noIssue"))
        .map_err(|err| format!("cannot create lock fixture directory: {err}"))?;
    fs::write(
        work.path()
            .join(".git/refs/remotes/origin/noIssue/locked.lock"),
        "stale lock\n",
    )
    .map_err(|err| format!("cannot write stale lock fixture: {err}"))?;

    let fake_fetch = work.path().join("fake-fetch.sh");
    fs::write(
        &fake_fetch,
        r#"#!/bin/sh
count_file="$1"
count=0
if [ -f "$count_file" ]; then count=$(cat "$count_file"); fi
count=$((count + 1))
printf '%s' "$count" > "$count_file"
if [ "$count" -lt 2 ]; then
  printf '%s\n' "error: could not delete references: cannot lock ref 'refs/remotes/origin/noIssue/locked': Unable to create '.git/refs/remotes/origin/noIssue/locked.lock': File exists." >&2
  printf '%s\n' "From example.invalid/repo" >&2
  printf '%s\n' " - [deleted]                   (none)                  -> origin/NOISSUE/deleted" >&2
  exit 1
fi
exit 0
"#,
    )
    .map_err(|err| format!("cannot write fake fetch script: {err}"))?;
    run_command(
        work.path(),
        "chmod",
        &["+x", fake_fetch.to_string_lossy().as_ref()],
    )?;

    let script = format!(
        "source {}/zsh/git-functions.zsh; _fetch_with_ref_cleanup \"$(git rev-parse --git-common-dir)\" origin {} .fetch-count",
        config_root.display(),
        fake_fetch.display(),
    );
    let zsh_output = run_command_output(work.path(), "zsh", &["-lc", &script])?;

    let debug_refs = run_command_output(work.path(), "git", &["show-ref"])?;
    if debug_refs.contains("NOISSUE/deleted") {
        return Err(format!(
            "zsh output:
{zsh_output}
debug refs after cleanup:
{debug_refs}"
        ));
    }
    assert_git_ref_missing(work.path(), "refs/remotes/origin/NOISSUE/deleted")?;
    assert_git_ref_missing(work.path(), "refs/remotes/origin/noIssue/locked")?;
    if work
        .path()
        .join(".git/refs/remotes/origin/noIssue/locked.lock")
        .exists()
    {
        return Err("stale remote-tracking ref lock was not removed".to_string());
    }
    let attempts = fs::read_to_string(work.path().join(".fetch-count"))
        .map_err(|err| format!("cannot read fake fetch attempt count: {err}"))?;
    if attempts.trim() != "2" {
        return Err(format!(
            "fake fetch should have succeeded on second attempt, got {attempts:?}"
        ));
    }
    Ok(())
}

fn test_terminal_title_config_and_format() -> Result<()> {
    let config_root = std::env::current_dir()
        .map_err(|err| format!("cannot determine config root for terminal title test: {err}"))?;
    let ghostty_config = fs::read_to_string(config_root.join("ghostty/config"))
        .map_err(|err| format!("cannot read Ghostty config for terminal title test: {err}"))?;
    if !ghostty_config
        .lines()
        .any(|line| line.trim() == "shell-integration-features = no-title")
    {
        return Err("Ghostty command-title integration should be disabled".to_string());
    }

    let work = tempfile::Builder::new()
        .prefix("tmp_terminal_title_repo_")
        .tempdir()
        .map_err(|err| format!("cannot create terminal title temp repo: {err}"))?;
    run_command(work.path(), "git", &["init", "--initial-branch=main"])?;
    fs::create_dir_all(work.path().join("packages/service/api"))
        .map_err(|err| format!("cannot create terminal title nested fixture: {err}"))?;

    let script = format!(
        "source {}/zsh/terminal-title.zsh; cd packages/service/api; __terminal_title_current",
        config_root.display()
    );
    let actual = run_command_output(work.path(), "zsh", &["-fc", &script])?;
    let repo = work
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "terminal title temp repo has no file name".to_string())?;
    let expected = format!("{repo}/…/service/api\n");

    if actual != expected {
        return Err(format!(
            "terminal title should include repo and compact relative cwd; got {actual:?}, expected {expected:?}"
        ));
    }

    Ok(())
}

fn assert_git_ref_missing(cwd: &Path, ref_name: &str) -> Result<()> {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(["show-ref", "--verify", "--quiet", ref_name])
        .output()
        .map_err(|err| format!("cannot run git show-ref for {ref_name}: {err}"))?;
    if output.status.success() {
        return Err(format!("{ref_name} should have been removed"));
    }
    Ok(())
}

fn run_command(cwd: &Path, program: &str, args: &[&str]) -> Result<()> {
    run_command_output(cwd, program, args).map(|_| ())
}

fn run_command_output(cwd: &Path, program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .current_dir(cwd)
        .args(args)
        .output()
        .map_err(|err| format!("cannot run {program}: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "command failed in {}: {} {}
stdout:
{}
stderr:
{}",
            cwd.display(),
            program,
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn copy_fixture() -> Result<TempDir> {
    let config_root = config_root_from_exe()?;
    let temp_dir = tempfile::Builder::new()
        .prefix("tmp_rovodev_skill_validation_")
        .tempdir()
        .map_err(|err| format!("cannot create temp fixture: {err}"))?;
    copy_tree(
        &config_root.join(".agents"),
        &temp_dir.path().join(".agents"),
    )?;
    copy_tree(
        &config_root.join("rovodev"),
        &temp_dir.path().join("rovodev"),
    )?;
    copy_tree(&config_root.join("zsh"), &temp_dir.path().join("zsh"))?;
    copy_tree(
        &config_root.join("ghostty"),
        &temp_dir.path().join("ghostty"),
    )?;
    Ok(temp_dir)
}

fn copy_tree(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)
        .map_err(|err| format!("{}: cannot create directory: {err}", target.display()))?;
    for entry in walkdir::WalkDir::new(source).follow_links(false) {
        let entry =
            entry.map_err(|err| format!("{}: cannot walk directory: {err}", source.display()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(source)
            .map_err(|err| format!("{}: cannot compute relative path: {err}", path.display()))?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let destination = target.join(relative);
        let file_type = entry.file_type();
        if file_type.is_dir() {
            fs::create_dir_all(&destination).map_err(|err| {
                format!("{}: cannot create directory: {err}", destination.display())
            })?;
        } else if file_type.is_symlink() {
            let link_target = fs::read_link(path)
                .map_err(|err| format!("{}: cannot read symlink: {err}", path.display()))?;
            #[cfg(unix)]
            unix_fs::symlink(&link_target, &destination).map_err(|err| {
                format!("{}: cannot create symlink: {err}", destination.display())
            })?;
            #[cfg(not(unix))]
            return Err(format!("{}: symlink fixtures require unix", path.display()));
        } else if file_type.is_file() {
            fs::copy(path, &destination)
                .map_err(|err| format!("{}: cannot copy file: {err}", path.display()))?;
        }
    }
    Ok(())
}

pub(crate) fn assert_clean_fixture_passes() -> Result<()> {
    let fixture = copy_fixture()?;
    generate_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
    ])?;
    let errors = validate_registry(fixture.path());
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "clean fixture validation failed:\n{}",
            errors.join("\n")
        ))
    }
}

fn create_codex_system_skills(home: &Path) -> Result<()> {
    for skill in ["skill-creator", "skill-installer", "openai-docs"] {
        let skill_dir = home.join(".codex/skills/.system").join(skill);
        fs::create_dir_all(&skill_dir).map_err(|err| {
            format!(
                "{}: cannot create Codex system skill: {err}",
                skill_dir.display()
            )
        })?;
        fs::write(skill_dir.join("SKILL.md"), format!("{skill} fixture\n"))
            .map_err(|err| format!("cannot write Codex system skill fixture: {err}"))?;
    }
    Ok(())
}

fn assert_symlink_resolves_to(link: &Path, expected: &Path) -> Result<()> {
    if !link.is_symlink() {
        return Err(format!("{}: expected a symlink", link.display()));
    }
    let actual = link
        .canonicalize()
        .map_err(|err| format!("{}: cannot resolve symlink: {err}", link.display()))?;
    let expected = expected.canonicalize().map_err(|err| {
        format!(
            "{}: cannot resolve expected target: {err}",
            expected.display()
        )
    })?;
    if actual != expected {
        return Err(format!(
            "{}: resolves to {}; expected {}",
            link.display(),
            actual.display(),
            expected.display()
        ));
    }
    Ok(())
}

fn assert_mutation_fails(name: &str, mutate: Mutation) -> Result<()> {
    let fixture = copy_fixture()?;
    let expected = mutate(fixture.path())?;
    let errors = validate_registry(fixture.path());
    let output = errors.join("\n");
    if errors.is_empty() {
        return Err(format!("{name}: validator unexpectedly passed\n{output}"));
    }
    if !output.contains(expected) {
        return Err(format!("{name}: expected {expected:?} in output\n{output}"));
    }
    Ok(())
}

fn load_config(config_root: &Path) -> Result<Value> {
    load_yaml_file(&config_root.join("rovodev/prompts.yml"))
}

fn write_config(config_root: &Path, config: &Value) -> Result<()> {
    let text = serde_yaml::to_string(config)
        .map_err(|err| format!("cannot render fixture config: {err}"))?;
    fs::write(
        config_root.join("rovodev/prompts.yml"),
        text.replace("---\n", ""),
    )
    .map_err(|err| format!("cannot write fixture config: {err}"))
}

fn first_prompt_name(config_root: &Path) -> Result<String> {
    let config = load_config(config_root)?;
    config
        .as_mapping()
        .and_then(|mapping| get(mapping, "prompts"))
        .and_then(Value::as_sequence)
        .and_then(|prompts| prompts.first())
        .and_then(Value::as_mapping)
        .and_then(|prompt| get(prompt, "name"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| "fixture prompts.yml is missing prompts[0].name".to_string())
}

fn mutate_missing_skill_file(config_root: &Path) -> Result<&'static str> {
    let name = first_prompt_name(config_root)?;
    fs::remove_file(
        config_root
            .join(".agents/skills")
            .join(name)
            .join("SKILL.md"),
    )
    .map_err(|err| format!("cannot remove fixture skill file: {err}"))?;
    Ok("content_file does not exist")
}

fn mutate_front_matter_name_mismatch(config_root: &Path) -> Result<&'static str> {
    let name = first_prompt_name(config_root)?;
    let skill_path = config_root
        .join(".agents/skills")
        .join(&name)
        .join("SKILL.md");
    let text = fs::read_to_string(&skill_path)
        .map_err(|err| format!("{}: cannot read fixture skill: {err}", skill_path.display()))?;
    fs::write(
        &skill_path,
        text.replacen(&format!("name: {name}"), "name: broken-name", 1),
    )
    .map_err(|err| {
        format!(
            "{}: cannot write fixture skill: {err}",
            skill_path.display()
        )
    })?;
    Ok("does not match directory")
}

fn mutate_generated_registry_drift(config_root: &Path) -> Result<&'static str> {
    let mut config = load_config(config_root)?;
    let prompts = config
        .as_mapping_mut()
        .and_then(|mapping| mapping.get_mut(string_key("prompts")))
        .and_then(Value::as_sequence_mut)
        .ok_or_else(|| "fixture prompts.yml is missing prompts".to_string())?;
    let prompt = prompts
        .iter_mut()
        .find(|prompt| {
            prompt
                .as_mapping()
                .and_then(|mapping| get(mapping, "name"))
                .and_then(Value::as_str)
                == Some("apply-changes")
        })
        .and_then(Value::as_mapping_mut)
        .ok_or_else(|| "fixture prompts.yml is missing apply-changes".to_string())?;
    let inputs = prompt
        .get_mut(string_key("inputs"))
        .and_then(Value::as_sequence_mut)
        .ok_or_else(|| "fixture apply-changes prompt is missing inputs".to_string())?;
    let first_input = inputs
        .first_mut()
        .and_then(Value::as_mapping_mut)
        .ok_or_else(|| "fixture apply-changes prompt is missing input mapping".to_string())?;
    first_input.insert(string_key("required"), Value::Bool(true));
    write_config(config_root, &config)?;
    Ok("generated content is not up to date")
}

fn mutate_front_matter_only_skill(config_root: &Path) -> Result<&'static str> {
    let name = first_prompt_name(config_root)?;
    let skill_path = config_root
        .join(".agents/skills")
        .join(&name)
        .join("SKILL.md");
    let text = fs::read_to_string(&skill_path)
        .map_err(|err| format!("{}: cannot read fixture skill: {err}", skill_path.display()))?;
    let closing = text
        .lines()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line == "---").then_some(index))
        .ok_or_else(|| {
            format!(
                "{}: fixture skill missing front matter",
                skill_path.display()
            )
        })?;
    let front_matter = text
        .lines()
        .take(closing + 1)
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&skill_path, format!("{front_matter}\n\n")).map_err(|err| {
        format!(
            "{}: cannot write fixture skill: {err}",
            skill_path.display()
        )
    })?;
    Ok("skill body must not be empty")
}

fn mutate_thin_skill_body(config_root: &Path) -> Result<&'static str> {
    let name = first_prompt_name(config_root)?;
    let skill_path = config_root
        .join(".agents/skills")
        .join(&name)
        .join("SKILL.md");
    let text = fs::read_to_string(&skill_path)
        .map_err(|err| format!("{}: cannot read fixture skill: {err}", skill_path.display()))?;
    let closing = text
        .lines()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line == "---").then_some(index))
        .ok_or_else(|| {
            format!(
                "{}: fixture skill missing front matter",
                skill_path.display()
            )
        })?;
    let front_matter = text
        .lines()
        .take(closing + 1)
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        &skill_path,
        format!("{front_matter}\n\nUse conversation context.\n"),
    )
    .map_err(|err| {
        format!(
            "{}: cannot write fixture skill: {err}",
            skill_path.display()
        )
    })?;
    Ok("skill body must contain at least")
}

fn mutate_placeholder_skill_body(config_root: &Path) -> Result<&'static str> {
    let name = first_prompt_name(config_root)?;
    let skill_path = config_root
        .join(".agents/skills")
        .join(&name)
        .join("SKILL.md");
    let text = fs::read_to_string(&skill_path)
        .map_err(|err| format!("{}: cannot read fixture skill: {err}", skill_path.display()))?;
    let closing = text
        .lines()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line == "---").then_some(index))
        .ok_or_else(|| {
            format!(
                "{}: fixture skill missing front matter",
                skill_path.display()
            )
        })?;
    let front_matter = text
        .lines()
        .take(closing + 1)
        .collect::<Vec<_>>()
        .join("\n");
    let body = "TODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\nTODO\n";
    fs::write(&skill_path, format!("{front_matter}\n\n{body}")).map_err(|err| {
        format!(
            "{}: cannot write fixture skill: {err}",
            skill_path.display()
        )
    })?;
    Ok("not a placeholder")
}

fn mutate_unregistered_skill_file(config_root: &Path) -> Result<&'static str> {
    let extra_dir = config_root.join(".agents/skills/unregistered-skill");
    fs::create_dir_all(&extra_dir).map_err(|err| {
        format!(
            "{}: cannot create fixture skill: {err}",
            extra_dir.display()
        )
    })?;
    fs::write(
        extra_dir.join("SKILL.md"),
        "---\nname: unregistered-skill\ndescription: Unregistered skill used by registry drift tests\n---\n\nRead the relevant fixture files and behave like a real skill so body validation succeeds before registry drift is checked.\nUse this body only to prove that adding a valid skill file without regenerating prompts.yml is rejected.\nKeep the instructions concrete enough that the failure comes from generated registry drift, not content quality.\n",
    )
    .map_err(|err| format!("cannot write fixture skill: {err}"))?;
    Ok("generated content is not up to date")
}

fn mutate_prompt_adapter_broken(config_root: &Path) -> Result<&'static str> {
    let prompts_link = config_root.join("rovodev/prompts");
    fs::remove_file(&prompts_link).map_err(|err| {
        format!(
            "{}: cannot remove fixture symlink: {err}",
            prompts_link.display()
        )
    })?;
    #[cfg(unix)]
    unix_fs::symlink("../.agents/missing", &prompts_link).map_err(|err| {
        format!(
            "{}: cannot create fixture symlink: {err}",
            prompts_link.display()
        )
    })?;
    #[cfg(not(unix))]
    return Err("broken prompt adapter fixture requires unix symlinks".to_string());
    Ok("must resolve to")
}
