use crate::commands::validate_command;
use crate::config_root::config_root_from_exe;
use crate::error::Result;
use crate::install::install_command;
use crate::registry::validate_registry;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
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
        ("front matter only skill", mutate_front_matter_only_skill),
        ("thin skill body", mutate_thin_skill_body),
        ("placeholder skill body", mutate_placeholder_skill_body),
    ];
    for (name, mutate) in mutations {
        assert_mutation_fails(name, *mutate)?;
    }
    test_new_skill_validate_flow()?;
    test_install_command()?;
    test_link_safety()?;
    test_command_failures()?;
    test_git_fetch_ref_cleanup()?;
    test_home_reset_to_origin()?;
    test_axiom_alias_zsh_wiring()?;
    test_relay_axiom_config()?;
    Ok(())
}

fn test_new_skill_validate_flow() -> Result<()> {
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
        "---\nname: real-e2e-fixture\ndescription: Fixture skill proving validation discovers new valid skills through the public command path.\nregister_cmd: true\n---\n\nDrive the public command path, not a helper-only shortcut, when proving newly added skills are discoverable.\nValidate the fixture checkout exactly as the normal config command does.\nThe fixture body is intentionally concrete so failures prove skill validation behavior rather than placeholder rejection.\n",
    )
    .map_err(|err| format!("cannot write fixture skill: {err}"))?;

    validate_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
    ])?;
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
        .prefix("tmp_config_install_home_")
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
        &home.path().join(".codex/skills/surgical-edit"),
        &fixture.path().join(".agents/skills/surgical-edit"),
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
    assert_symlink_resolves_to(
        &home.path().join(".relay/config.toml"),
        &fixture.path().join("relay/config.toml"),
    )?;
    let installed_binary = home.path().join(".local/bin/config-tools");
    if !installed_binary.is_file() {
        return Err(format!(
            "install should create runnable config-tools binary at {}",
            installed_binary.display()
        ));
    }
    #[cfg(unix)]
    if fs::metadata(&installed_binary)
        .map_err(|err| format!("cannot inspect installed config-tools binary: {err}"))?
        .permissions()
        .mode()
        & 0o111
        == 0
    {
        return Err("installed config-tools binary should be executable".to_string());
    }
    run_command(
        home.path(),
        installed_binary.to_string_lossy().as_ref(),
        &["--help"],
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
        .join(".codex/skills/surgical-edit");
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

    let install_fixture = copy_fixture()?;
    let home = tempfile::Builder::new()
        .prefix("tmp_config_install_safety_")
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

    let binary_conflict_fixture = copy_fixture()?;
    let binary_conflict_home = tempfile::Builder::new()
        .prefix("tmp_config_install_binary_conflict_")
        .tempdir()
        .map_err(|err| format!("cannot create binary conflict home: {err}"))?;
    create_codex_system_skills(binary_conflict_home.path())?;
    let binary_conflict = binary_conflict_home.path().join(".local/bin/config-tools");
    fs::create_dir_all(
        binary_conflict
            .parent()
            .ok_or("binary target has no parent")?,
    )
    .map_err(|err| format!("cannot create binary conflict parent: {err}"))?;
    fs::write(&binary_conflict, "do not replace")
        .map_err(|err| format!("cannot write binary conflict fixture: {err}"))?;
    let binary_conflict_error = install_command(&[
        "--config-root".to_string(),
        binary_conflict_fixture.path().display().to_string(),
        "--home".to_string(),
        binary_conflict_home.path().display().to_string(),
    ])
    .expect_err("install should reject unrelated config-tools binary target");
    if !binary_conflict_error.contains("managed config-tools binary") {
        return Err(format!(
            "unexpected binary conflict error: {binary_conflict_error}"
        ));
    }
    if fs::read_to_string(&binary_conflict)
        .map_err(|err| format!("cannot read binary conflict fixture: {err}"))?
        != "do not replace"
    {
        return Err("install replaced unrelated config-tools binary target".to_string());
    }
    Ok(())
}

pub(crate) fn test_command_failures() -> Result<()> {
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
    fs::write(work.path().join("file.txt"), "base\n")
        .map_err(|err| format!("cannot write git fixture file: {err}"))?;
    run_command(work.path(), "git", &["add", "file.txt"])?;
    run_command(work.path(), "git", &["commit", "-m", "base"])?;
    run_command(work.path(), "git", &["checkout", "-b", "feature"])?;
    fs::write(work.path().join("file.txt"), "feature\n")
        .map_err(|err| format!("cannot write feature fixture file: {err}"))?;
    run_command(work.path(), "git", &["commit", "-am", "feature"])?;
    let remote = tempfile::Builder::new()
        .prefix("tmp_git_fetch_ref_cleanup_remote_")
        .tempdir()
        .map_err(|err| format!("cannot create temp remote: {err}"))?;
    run_command(remote.path(), "git", &["init", "--bare"])?;
    run_command(
        work.path(),
        "git",
        &[
            "remote",
            "add",
            "origin",
            remote.path().to_string_lossy().as_ref(),
        ],
    )?;
    run_command(work.path(), "git", &["push", "-u", "origin", "main"])?;
    run_command(work.path(), "git", &["push", "-u", "origin", "feature"])?;

    run_command(work.path(), "git", &["checkout", "main"])?;
    run_command(work.path(), "git", &["fetch", "origin", "feature"])?;
    assert_ref_exists(work.path(), "FETCH_HEAD")?;
    assert_ref_exists(work.path(), "refs/remotes/origin/feature")?;

    let script = format!(
        r#"source "{}"
git update-ref -d refs/remotes/origin/feature || exit $?
"#,
        config_root.join("zsh/git-functions.zsh").display()
    );
    run_command(work.path(), "zsh", &["-c", &script])?;
    assert_ref_missing(work.path(), "refs/remotes/origin/feature")?;
    Ok(())
}

fn test_home_reset_to_origin() -> Result<()> {
    let config_root = std::env::current_dir()
        .map_err(|err| format!("cannot determine config root for zsh alias test: {err}"))?;
    let script = format!(
        r#"source "{}"
functions home_reset_to_origin >/dev/null
"#,
        config_root.join("zsh/git-functions.zsh").display()
    );
    run_command(&config_root, "zsh", &["-c", &script])
}

fn test_axiom_alias_zsh_wiring() -> Result<()> {
    let config_root = std::env::current_dir()
        .map_err(|err| format!("cannot determine config root for zsh alias test: {err}"))?;
    let script = format!(
        r#"source "{}"
alias axiom >/dev/null
alias atlassian >/dev/null
alias alta-1 >/dev/null
alias alta-2 >/dev/null
alias alta-3 >/dev/null
alias alta-4 >/dev/null
alias alta-5 >/dev/null
"#,
        config_root.join("zsh/zshrc").display()
    );
    run_command(&config_root, "zsh", &["-c", &script])
}

fn test_relay_axiom_config() -> Result<()> {
    let config_root = std::env::current_dir()
        .map_err(|err| format!("cannot determine config root for relay config test: {err}"))?;
    let config_path = config_root.join("relay/config.toml");
    let text = fs::read_to_string(&config_path)
        .map_err(|err| format!("{}: cannot read Relay config: {err}", config_path.display()))?;
    for expected in [
        "[agents]",
        "default = \"axiom\"",
        "[agents.runners.axiom]",
        "command = \"/opt/atlassian/bin/atlas\"",
        "args = [\"alta\", \"agent\", \"run\", \"@atlassian/alta-agent-axiom-asumer\"]",
    ] {
        if !text.contains(expected) {
            return Err(format!(
                "{}: Relay config missing expected text {expected:?}",
                config_path.display()
            ));
        }
    }
    Ok(())
}

fn assert_ref_exists(cwd: &Path, ref_name: &str) -> Result<()> {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(["show-ref", "--verify", "--quiet", ref_name])
        .output()
        .map_err(|err| format!("cannot run git show-ref for {ref_name}: {err}"))?;
    if !output.status.success() {
        return Err(format!("{ref_name} should exist"));
    }
    Ok(())
}

fn assert_ref_missing(cwd: &Path, ref_name: &str) -> Result<()> {
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
        .prefix("tmp_config_skill_validation_")
        .tempdir()
        .map_err(|err| format!("cannot create temp fixture: {err}"))?;
    copy_tree(
        &config_root.join(".agents"),
        &temp_dir.path().join(".agents"),
    )?;
    copy_tree(&config_root.join("zsh"), &temp_dir.path().join("zsh"))?;
    copy_tree(
        &config_root.join("ghostty"),
        &temp_dir.path().join("ghostty"),
    )?;
    copy_tree(&config_root.join("relay"), &temp_dir.path().join("relay"))?;
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

fn first_skill_name(config_root: &Path) -> Result<String> {
    for entry in fs::read_dir(config_root.join(".agents/skills"))
        .map_err(|err| format!("cannot read fixture skills: {err}"))?
    {
        let entry = entry.map_err(|err| format!("cannot inspect fixture skill: {err}"))?;
        if entry
            .file_type()
            .map_err(|err| format!("cannot inspect fixture skill type: {err}"))?
            .is_dir()
            && !entry.file_name().to_string_lossy().starts_with('.')
            && entry.path().join("SKILL.md").is_file()
        {
            return Ok(entry.file_name().to_string_lossy().to_string());
        }
    }
    Err("fixture is missing skills".to_string())
}

fn mutate_missing_skill_file(config_root: &Path) -> Result<&'static str> {
    for entry in fs::read_dir(config_root.join(".agents/skills"))
        .map_err(|err| format!("cannot read fixture skills: {err}"))?
    {
        let entry = entry.map_err(|err| format!("cannot inspect fixture skill: {err}"))?;
        if !entry
            .file_type()
            .map_err(|err| format!("cannot inspect fixture skill type: {err}"))?
            .is_dir()
            || entry.file_name().to_string_lossy().starts_with('.')
        {
            continue;
        }
        let skill_file = entry.path().join("SKILL.md");
        if skill_file.is_file() {
            fs::remove_file(&skill_file).map_err(|err| {
                format!(
                    "{}: cannot remove fixture skill file: {err}",
                    skill_file.display()
                )
            })?;
        }
    }
    Ok("no skills found")
}

fn mutate_front_matter_name_mismatch(config_root: &Path) -> Result<&'static str> {
    let name = first_skill_name(config_root)?;
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

fn mutate_front_matter_only_skill(config_root: &Path) -> Result<&'static str> {
    let name = first_skill_name(config_root)?;
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
    let name = first_skill_name(config_root)?;
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
    let name = first_skill_name(config_root)?;
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
