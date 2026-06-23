use crate::commands::validate_command;
use crate::config_root::config_root_from_exe;
use crate::error::Result;
use crate::install::{check_codex_skills_command, check_install_command, install_command};
use crate::managed_config::validate_managed_configs;
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
            "missing required custom skill",
            mutate_missing_required_custom_skill,
        ),
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
    test_real_e2e_automated_tests_skill_requires_edge_case_proof()?;
    test_real_e2e_live_check_skill_rejects_test_lane_proof()?;
    test_git_publish_skills_keep_local_first_contract()?;
    test_ghostty_config_rejects_tab_disappearance_regressions()?;
    test_install_command()?;
    test_link_safety()?;
    test_command_failures()?;
    test_git_fetch_ref_cleanup()?;
    test_get_default_branch_refreshes_stale_remote_head()?;
    test_home_reset_to_origin()?;
    test_axiom_alias_zsh_wiring()?;
    test_relay_axiom_config()?;
    Ok(())
}

fn test_ghostty_config_rejects_tab_disappearance_regressions() -> Result<()> {
    let fixture = copy_fixture()?;
    let clean_errors = validate_managed_configs(fixture.path());
    if !clean_errors.is_empty() {
        return Err(format!(
            "clean Ghostty config should pass managed-config validation:\n{}",
            clean_errors.join("\n")
        ));
    }

    for (name, line, expected) in [
        (
            "titlebar tabs",
            "macos-titlebar-style = tabs",
            "must not set `macos-titlebar-style` to `tabs`",
        ),
        (
            "non-native fullscreen",
            "macos-non-native-fullscreen = true",
            "must not set `macos-non-native-fullscreen` to `true`",
        ),
        (
            "forced window restore",
            "window-save-state = always",
            "must not set `window-save-state` to `always`",
        ),
    ] {
        let fixture = copy_fixture()?;
        let ghostty_config = fixture.path().join("ghostty/config");
        let mut text = fs::read_to_string(&ghostty_config).map_err(|err| {
            format!(
                "{}: cannot read fixture Ghostty config: {err}",
                ghostty_config.display()
            )
        })?;
        text.push('\n');
        text.push_str(line);
        text.push('\n');
        fs::write(&ghostty_config, text).map_err(|err| {
            format!(
                "{}: cannot write fixture Ghostty config: {err}",
                ghostty_config.display()
            )
        })?;

        let errors = validate_managed_configs(fixture.path());
        let output = errors.join("\n");
        if !output.contains(expected) {
            return Err(format!(
                "{name}: Ghostty managed-config guard missed {expected:?}\n{output}"
            ));
        }
    }

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
        "---\nname: real-e2e-fixture\ndescription: Fixture skill proving validation discovers new valid skills through the public command path.\n---\n\nDrive the public command path, not a helper-only shortcut, when proving newly added skills are discoverable.\nValidate the fixture checkout exactly as the normal config command does.\nThe fixture body is intentionally concrete so failures prove skill validation behavior rather than placeholder rejection.\n",
    )
    .map_err(|err| format!("cannot write fixture skill: {err}"))?;

    validate_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
    ])?;
    Ok(())
}

fn test_real_e2e_automated_tests_skill_requires_edge_case_proof() -> Result<()> {
    let config_root = config_root_from_exe()?;
    let skill_path = config_root.join(".agents/skills/real-e2e-automated-tests/SKILL.md");
    let text = fs::read_to_string(&skill_path).map_err(|err| {
        format!(
            "{}: cannot read real-e2e-automated-tests skill: {err}",
            skill_path.display()
        )
    })?;
    for expected in [
        "Repo Discovery Protocol",
        "source-backed route map",
        "Realness contract is binary",
        "Confidence is achieved only for the exact checked contract",
        "Persistence standard",
        "Blocked is a last resort",
        "not yet a blocker",
        "perform blocker burn-down",
        "Report blocked only when no safe source-backed next action remains",
        "request required approval",
        "Completion gate before final",
        "keep going or report blocked only after blocker burn-down; never produce a complete confidence line",
        "no unresolved realness-critical unknown",
        "real public boundary",
        "Realistic regression proof failed for the expected reason",
        "Never hand-roll deployed auth",
        "Before adding a new E2E lane, prove no existing lane owns the boundary",
        "package-script-name guesses",
        "identify a small edge-case matrix",
        "automated E2E must include at least one meaningful edge/negative assertion",
        "happy-path-only E2E is incomplete",
        "edge checks performed only against mocks/fakes",
        "edge cases intentionally omitted with reasons",
        "Confidence: complete for the stated automated E2E contract",
        "Confidence: not achieved because <blocker>",
    ] {
        if !text.contains(expected) {
            return Err(format!(
                "{}: real-e2e-automated-tests skill must require robust edge-case proof; missing {expected:?}",
                skill_path.display()
            ));
        }
    }
    Ok(())
}

fn test_real_e2e_live_check_skill_rejects_test_lane_proof() -> Result<()> {
    let config_root = config_root_from_exe()?;
    let skill_path = config_root.join(".agents/skills/real-e2e-live-check/SKILL.md");
    let text = fs::read_to_string(&skill_path).map_err(|err| {
        format!(
            "{}: cannot read real-e2e-live-check skill: {err}",
            skill_path.display()
        )
    })?;
    for expected in [
        "Repo Discovery Protocol",
        "source-backed route map",
        "Realness contract is binary",
        "Confidence is achieved only for the exact checked contract",
        "Persistence standard",
        "Blocked is a last resort",
        "not yet a blocker",
        "perform blocker burn-down",
        "Report blocked only when no safe source-backed next action remains",
        "request required approval",
        "Completion gate before final",
        "keep going or report blocked only after blocker burn-down; never produce a complete confidence line",
        "no unresolved realness-critical unknown",
        "Real public boundary operated",
        "Post-operation health",
        "Never hand-roll deployed auth",
        "Do not proceed from package-script names alone",
        "without writing automated tests",
        "without running CI/test lanes",
        "Environment selection ladder",
        "safety class",
        "Choose an agent protocol client only when the behavior is observable through the agent protocol",
        "Report blocked instead",
        "Confidence: complete for the stated live-check contract",
        "Confidence: not achieved because <blocker>",
    ] {
        if !text.contains(expected) {
            return Err(format!(
                "{}: real-e2e-live-check skill must reject test-lane proof and require safe live evidence; missing {expected:?}",
                skill_path.display()
            ));
        }
    }
    Ok(())
}

fn test_git_publish_skills_keep_local_first_contract() -> Result<()> {
    let config_root = config_root_from_exe()?;
    let to_origin_path = config_root.join(".agents/skills/git-publish-to-origin/SKILL.md");
    let to_origin = fs::read_to_string(&to_origin_path).map_err(|err| {
        format!(
            "{}: cannot read git-publish-to-origin skill: {err}",
            to_origin_path.display()
        )
    })?;
    let publish_path = config_root.join(".agents/skills/git-publish/SKILL.md");
    let publish = fs::read_to_string(&publish_path).map_err(|err| {
        format!(
            "{}: cannot read git-publish skill: {err}",
            publish_path.display()
        )
    })?;

    for expected in [
        "This skill is push-only: do not create branches, open PRs, inspect PRs, update PRs",
        "Normal path contract: local-first, direct push, push-verified",
        "Do not run remote-default discovery in the normal path",
        "Treat the actual `git push` result as the authoritative network freshness and safety check",
        "Resolve the push target directly as branch `<current-branch>` on `origin`",
        "local push target evidence: `git remote get-url --push --all origin`",
        "Push `HEAD` to branch `<current-branch>` on `origin`",
    ] {
        if !to_origin.contains(expected) {
            return Err(format!(
                "{}: git-publish-to-origin must stay a direct local-first push workflow; missing {expected:?}",
                to_origin_path.display()
            ));
        }
    }
    for forbidden in [
        "remote default branch from",
        "`git remote show origin`",
        "`git ls-remote --symref origin HEAD`",
        "Inspect canonical PR context",
        "ensure_bitbucket_pr",
        "twg bb prs create",
        "gh pr create",
    ] {
        if to_origin.contains(forbidden) {
            return Err(format!(
                "{}: git-publish-to-origin must not make direct push depend on PR/default-discovery workflow text; found {forbidden:?}",
                to_origin_path.display()
            ));
        }
    }

    for expected in [
        "local-first default branch resolver",
        "Use `refs/remotes/origin/HEAD` only when it resolves to an existing local `refs/remotes/origin/<branch>` ref.",
        "Otherwise use a local `origin/main` or `origin/master` candidate only when exactly one exists.",
        "Run live remote discovery only when a PR destination is required and local refs are missing, stale, or conflicting.",
        "Determine the PR provider from `git remote get-url --push origin`",
        "After the source branch has been pushed",
        "Do not add reviewers unless the user explicitly requested reviewers.",
        "Save fresh PR metadata",
        "Ensure the PR through `ensure_bitbucket_pr` with explicit source and destination branches and no reviewers unless reviewers were explicitly requested.",
    ] {
        if !publish.contains(expected) {
            return Err(format!(
                "{}: git-publish must keep PR-grade local-first destination safety; missing {expected:?}",
                publish_path.display()
            ));
        }
    }
    for forbidden in [
        "usually from `git remote show origin`",
        "`git remote show origin`",
        "`git ls-remote --symref origin HEAD`",
    ] {
        if publish.contains(forbidden) {
            return Err(format!(
                "{}: git-publish must not describe expensive live default discovery as the usual resolver; found {forbidden:?}",
                publish_path.display()
            ));
        }
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
        .prefix("tmp_config_install_home_")
        .tempdir()
        .map_err(|err| format!("cannot create temp install home: {err}"))?;
    create_codex_system_skills(home.path())?;
    fs::create_dir_all(home.path().join(".relay"))
        .map_err(|err| format!("cannot create legacy Relay config fixture dir: {err}"))?;
    #[cfg(unix)]
    unix_fs::symlink(
        fixture.path().join("relay/config.toml"),
        home.path().join(".relay/config.toml"),
    )
    .map_err(|err| format!("cannot create legacy Relay config fixture symlink: {err}"))?;
    #[cfg(not(unix))]
    fs::copy(
        fixture.path().join("relay/config.toml"),
        home.path().join(".relay/config.toml"),
    )
    .map_err(|err| format!("cannot create legacy Relay config fixture file: {err}"))?;
    let codex_config = home.path().join(".codex/config.toml");
    fs::write(
        &codex_config,
        "model = \"gpt-5.5\"\n\n[features]\n  codex_hooks = true\n  hooks = true\n  apps = false\n",
    )
    .map_err(|err| {
        format!(
            "{}: cannot write Codex config fixture: {err}",
            codex_config.display()
        )
    })?;
    let stale_managed_skill = home.path().join(".codex/skills/review-code");
    let external_skill_dir = home.path().join("external-skill");
    fs::create_dir(&external_skill_dir)
        .map_err(|err| format!("cannot create external skill fixture: {err}"))?;
    #[cfg(unix)]
    {
        unix_fs::symlink(
            fixture.path().join(".agents/skills/review-code"),
            &stale_managed_skill,
        )
        .map_err(|err| format!("cannot create stale managed skill symlink: {err}"))?;
        unix_fs::symlink(
            &external_skill_dir,
            home.path().join(".codex/skills/external-skill"),
        )
        .map_err(|err| format!("cannot create external skill symlink: {err}"))?;
    }

    let initial_install_drift = check_install_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])
    .expect_err("install drift check should fail before install converges managed config");
    for expected in [
        "Managed config installation drift detected",
        ".config/relay/config.toml",
        ".relay remains but Relay now reads ~/.config/relay",
        "deprecated [features].codex_hooks must be removed",
        "[features].apps must not be forced off",
        "is a stale managed Codex skill symlink",
        "is missing; expected symlink",
    ] {
        if !initial_install_drift.contains(expected) {
            return Err(format!(
                "initial install drift check missed {expected:?}: {initial_install_drift}"
            ));
        }
    }

    let initial_codex_drift = check_codex_skills_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])
    .expect_err("Codex skill drift check should fail before install converges custom links");
    if !initial_codex_drift.contains("Custom Codex skill installation drift detected") {
        return Err(format!(
            "initial Codex skill drift check reported unexpected output: {initial_codex_drift}"
        ));
    }

    install_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])?;
    let repaired_codex_config = fs::read_to_string(&codex_config).map_err(|err| {
        format!(
            "{}: cannot read repaired Codex config: {err}",
            codex_config.display()
        )
    })?;
    if repaired_codex_config.contains("codex_hooks")
        || repaired_codex_config.contains("apps = false")
    {
        return Err(format!(
            "install did not repair deprecated/disabled Codex config flags:\n{repaired_codex_config}"
        ));
    }

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
        &home.path().join(".config/relay/config.toml"),
        &fixture.path().join("relay/config.toml"),
    )?;
    if home.path().join(".relay").exists() {
        return Err("install should remove empty legacy ~/.relay config directory".to_string());
    }
    check_install_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])?;
    let installed_binary = home.path().join(".local/bin/config-tools");
    if !installed_binary.is_file() {
        return Err(format!(
            "install should create runnable config-tools binary at {}",
            installed_binary.display()
        ));
    }
    let installed_codex_launcher = home.path().join(".local/bin/codex");
    let installed_codex_launcher_text =
        fs::read_to_string(&installed_codex_launcher).map_err(|err| {
            format!(
                "{}: cannot read installed Codex launcher: {err}",
                installed_codex_launcher.display()
            )
        })?;
    for expected in ["repair-codex-config", "/opt/homebrew/bin/codex"] {
        if !installed_codex_launcher_text.contains(expected) {
            return Err(format!(
                "installed Codex launcher missed {expected:?}:\n{installed_codex_launcher_text}"
            ));
        }
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
    #[cfg(unix)]
    if fs::metadata(&installed_codex_launcher)
        .map_err(|err| format!("cannot inspect installed Codex launcher: {err}"))?
        .permissions()
        .mode()
        & 0o111
        == 0
    {
        return Err("installed Codex launcher should be executable".to_string());
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
    if stale_managed_skill.exists() || stale_managed_skill.is_symlink() {
        return Err("install left a stale managed Codex skill symlink behind".to_string());
    }
    assert_symlink_resolves_to(
        &home.path().join(".codex/skills/external-skill"),
        &external_skill_dir,
    )?;
    check_codex_skills_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])?;

    let removed_skill = home.path().join(".codex/skills/removed-skill");
    #[cfg(unix)]
    unix_fs::symlink(
        fixture.path().join(".agents/skills/removed-skill"),
        &removed_skill,
    )
    .map_err(|err| format!("cannot create removed managed skill symlink: {err}"))?;
    let stale_drift = check_codex_skills_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])
    .expect_err("Codex skill drift check should fail when a removed managed skill link remains");
    if !stale_drift.contains("removed-skill")
        || !stale_drift.contains("is a stale managed Codex skill symlink")
    {
        return Err(format!(
            "stale managed Codex skill drift check missed removed-skill: {stale_drift}"
        ));
    }

    install_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])?;
    check_codex_skills_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])?;

    let missing_source_skill = fixture.path().join(".agents/skills/address-comments");
    fs::remove_dir_all(&missing_source_skill).map_err(|err| {
        format!(
            "{}: cannot remove required source skill fixture: {err}",
            missing_source_skill.display()
        )
    })?;
    let invalid_source_check = check_codex_skills_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])
    .expect_err(
        "Codex skill drift check should fail when source registry is missing a required skill",
    );
    if !invalid_source_check.contains("missing required custom skills: address-comments") {
        return Err(format!(
            "Codex skill drift check did not reject invalid source registry: {invalid_source_check}"
        ));
    }
    let invalid_source_install = install_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])
    .expect_err(
        "install should fail before mutating when source registry is missing a required skill",
    );
    if !invalid_source_install.contains("missing required custom skills: address-comments") {
        return Err(format!(
            "install did not reject invalid source registry: {invalid_source_install}"
        ));
    }
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
    if !validate_error.contains("Config validation failed") {
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

fn test_get_default_branch_refreshes_stale_remote_head() -> Result<()> {
    let config_root = std::env::current_dir()
        .map_err(|err| format!("cannot determine config root for default branch test: {err}"))?;
    if !config_root.join("zsh/git-functions.zsh").is_file() {
        return Err(format!(
            "{}: default branch test must run from config repo root",
            config_root.display()
        ));
    }

    let seed = tempfile::Builder::new()
        .prefix("tmp_default_branch_seed_")
        .tempdir()
        .map_err(|err| format!("cannot create temp seed repo: {err}"))?;
    let remote = tempfile::Builder::new()
        .prefix("tmp_default_branch_remote_")
        .tempdir()
        .map_err(|err| format!("cannot create temp remote repo: {err}"))?;
    let work_parent = tempfile::Builder::new()
        .prefix("tmp_default_branch_work_")
        .tempdir()
        .map_err(|err| format!("cannot create temp work parent: {err}"))?;
    let work = work_parent.path().join("work");

    run_command(seed.path(), "git", &["init", "--initial-branch=master"])?;
    run_command(
        seed.path(),
        "git",
        &["config", "user.email", "test@example.com"],
    )?;
    run_command(seed.path(), "git", &["config", "user.name", "Test User"])?;
    fs::write(seed.path().join("file.txt"), "master\n")
        .map_err(|err| format!("cannot write seed fixture file: {err}"))?;
    run_command(seed.path(), "git", &["add", "file.txt"])?;
    run_command(seed.path(), "git", &["commit", "-m", "master"])?;

    run_command(remote.path(), "git", &["init", "--bare"])?;
    run_command(
        remote.path(),
        "git",
        &["symbolic-ref", "HEAD", "refs/heads/master"],
    )?;
    run_command(
        seed.path(),
        "git",
        &[
            "remote",
            "add",
            "origin",
            remote.path().to_string_lossy().as_ref(),
        ],
    )?;
    run_command(seed.path(), "git", &["push", "-u", "origin", "master"])?;
    run_command(
        work_parent.path(),
        "git",
        &[
            "clone",
            remote.path().to_string_lossy().as_ref(),
            work.to_string_lossy().as_ref(),
        ],
    )?;

    run_command(seed.path(), "git", &["checkout", "-b", "main"])?;
    fs::write(seed.path().join("file.txt"), "main\n")
        .map_err(|err| format!("cannot update seed fixture file: {err}"))?;
    run_command(seed.path(), "git", &["commit", "-am", "main"])?;
    run_command(seed.path(), "git", &["push", "-u", "origin", "main"])?;
    run_command(
        remote.path(),
        "git",
        &["symbolic-ref", "HEAD", "refs/heads/main"],
    )?;
    run_command(&work, "git", &["fetch", "origin", "main"])?;

    let stale_head =
        run_command_output(&work, "git", &["symbolic-ref", "refs/remotes/origin/HEAD"])?;
    if stale_head.trim() != "refs/remotes/origin/master" {
        return Err(format!(
            "fixture should start with stale origin/HEAD; got {stale_head:?}"
        ));
    }

    let script = format!(
        r#"source "{}"
default_branch=$(_get_default_branch origin) || exit $?
remote_head=$(git symbolic-ref refs/remotes/origin/HEAD) || exit $?
[[ "$default_branch" == "main" ]] || {{
    print -r -- "expected main, got $default_branch"
    exit 1
}}
[[ "$remote_head" == "refs/remotes/origin/main" ]] || {{
    print -r -- "expected refreshed origin/HEAD, got $remote_head"
    exit 1
}}
"#,
        config_root.join("zsh/git-functions.zsh").display()
    );
    run_command(&work, "zsh", &["-c", &script])?;
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
    let home = tempfile::Builder::new()
        .prefix("tmp_config_zsh_home_")
        .tempdir()
        .map_err(|err| format!("cannot create temp HOME for zsh alias test: {err}"))?;

    for dir in [
        ".oh-my-zsh",
        ".zsh",
        "atlassian/alta-1",
        "atlassian/alta-2",
        "atlassian/relay-3",
        "atlassian/relay-3-old",
        "atlassian/alta-contrib-3",
        "atlassian/alta-contrib-3x",
        "atlassian/convo-ai-3",
        "atlassian/convo-ai-test",
        "atlassian/sandboxes",
        "src/alexandersumer.com",
    ] {
        fs::create_dir_all(home.path().join(dir))
            .map_err(|err| format!("cannot create zsh alias fixture dir {dir}: {err}"))?;
    }
    fs::write(home.path().join(".oh-my-zsh/oh-my-zsh.sh"), "")
        .map_err(|err| format!("cannot write zsh alias fixture oh-my-zsh shim: {err}"))?;
    fs::write(home.path().join(".zsh/git-functions.zsh"), "")
        .map_err(|err| format!("cannot write zsh alias fixture git-functions shim: {err}"))?;

    let script = format!(
        r#"expect_alias() {{
  local name="$1"
  local expected="$2"
  local actual
  actual=$(alias "$name") || return $?
  if [[ "$actual" != "$name='$expected'" ]]; then
    printf 'expected alias %s=%q, got %s\n' "$name" "$expected" "$actual" >&2
    return 1
  fi
}}

source "{}"
expect_alias axiom "atlas relay --agent axiom"
expect_alias atlassian "cd $HOME/atlassian"
expect_alias blog "cd $HOME/src/alexandersumer.com"
expect_alias atlassian-sandbox "cd $HOME/atlassian/sandboxes"
expect_alias alta-1 "cd $HOME/atlassian/alta-1"
expect_alias alta-2 "cd $HOME/atlassian/alta-2"
expect_alias relay-3 "cd $HOME/atlassian/relay-3"
expect_alias alta-contrib-3 "cd $HOME/atlassian/alta-contrib-3"
expect_alias convo-ai-3 "cd $HOME/atlassian/convo-ai-3 && sdk use java 21.0.8-amzn"
! alias relay-3-old >/dev/null 2>&1
! alias alta-contrib-3x >/dev/null 2>&1
! alias convo-ai-test >/dev/null 2>&1
"#,
        config_root.join("zsh/zshrc").display()
    );
    let output = Command::new("zsh")
        .current_dir(&config_root)
        .env("HOME", home.path())
        .args(["-c", &script])
        .output()
        .map_err(|err| format!("cannot run zsh alias test: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "zsh alias test failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    Ok(())
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
    for skill in ["skill-creator", "skill-installer"] {
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

fn mutate_missing_required_custom_skill(config_root: &Path) -> Result<&'static str> {
    let skill_dir = config_root.join(".agents/skills/address-comments");
    fs::remove_dir_all(&skill_dir).map_err(|err| {
        format!(
            "{}: cannot remove required fixture skill directory: {err}",
            skill_dir.display()
        )
    })?;
    Ok("missing required custom skills: address-comments")
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
