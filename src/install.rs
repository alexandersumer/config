use crate::config_root::default_config_root;
use crate::error::Result;
use crate::links::{link_path, require_dir};
use crate::registry::validate_registry;
use std::collections::BTreeSet;
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn install_command(args: &[String]) -> Result<()> {
    let (config_root, home_dir) = parse_install_args(args)?;
    let agents_dir = config_root.join(".agents");
    let skills_dir = agents_dir.join("skills");
    let global_agents_dir = home_dir.join(".agents");
    let zsh_dir = config_root.join("zsh");
    let ghostty_dir = config_root.join("ghostty");
    let relay_dir = config_root.join("relay");

    require_dir(&agents_dir, "config agents directory")?;
    require_dir(&skills_dir, "config skills directory")?;
    validate_config_skills(&config_root)?;
    require_dir(&zsh_dir, "config zsh directory")?;
    require_dir(&ghostty_dir, "config Ghostty directory")?;
    require_dir(&relay_dir, "config Relay directory")?;
    let codex_skill_plan = prepare_codex_skill_links(&skills_dir, &home_dir)?;
    let claude_skill_plan = prepare_claude_skill_links(&skills_dir, &home_dir)?;
    repair_codex_config(&home_dir)?;
    fs::create_dir_all(home_dir.join(".config/ghostty")).map_err(|err| {
        format!(
            "{}: cannot create Ghostty config directory: {err}",
            home_dir.join(".config/ghostty").display()
        )
    })?;
    fs::create_dir_all(home_dir.join(".config/relay")).map_err(|err| {
        format!(
            "{}: cannot create Relay config directory: {err}",
            home_dir.join(".config/relay").display()
        )
    })?;

    link_path(
        &agents_dir,
        &global_agents_dir,
        "global agents directory",
        &config_root,
    )?;
    link_path(
        &zsh_dir,
        &home_dir.join(".zsh"),
        "zsh config directory",
        &config_root,
    )?;
    link_path(
        &zsh_dir.join("zshrc"),
        &home_dir.join(".zshrc"),
        "zshrc",
        &config_root,
    )?;
    link_path(
        &ghostty_dir.join("config"),
        &home_dir.join(".config/ghostty/config"),
        "Ghostty config",
        &config_root,
    )?;
    cleanup_duplicate_ghostty_app_config(&home_dir, &ghostty_dir.join("config"))?;
    link_path(
        &relay_dir.join("config.toml"),
        &home_dir.join(".config/relay/config.toml"),
        "Relay config",
        &config_root,
    )?;
    cleanup_legacy_relay_config(&home_dir, &relay_dir.join("config.toml"))?;
    apply_skill_links(codex_skill_plan)?;
    let claude_skills_dir = home_dir.join(".claude/skills");
    fs::create_dir_all(&claude_skills_dir).map_err(|err| {
        format!(
            "{}: cannot create Claude Code skills directory: {err}",
            claude_skills_dir.display()
        )
    })?;
    apply_skill_links(claude_skill_plan)?;
    install_config_tools_binary(&home_dir)?;
    install_codex_launcher(&home_dir)?;

    println!();
    println!(
        "Done. Agent config is now managed from {}.",
        config_root.display()
    );
    println!("~/.agents points at {}.", agents_dir.display());
    println!("Custom Codex skills point at {}.", skills_dir.display());
    println!(
        "Custom Claude Code skills point at {}.",
        skills_dir.display()
    );
    println!(
        "Relay config points at {}.",
        relay_dir.join("config.toml").display()
    );
    Ok(())
}

fn cleanup_duplicate_ghostty_app_config(
    home_dir: &Path,
    managed_ghostty_config: &Path,
) -> Result<()> {
    let app_config_dir = home_dir.join("Library/Application Support/com.mitchellh.ghostty");
    let app_config = app_config_dir.join("config");

    if config_matches_managed(&app_config, managed_ghostty_config)? {
        fs::remove_file(&app_config).map_err(|err| {
            format!(
                "{}: cannot remove duplicate Ghostty app config: {err}",
                app_config.display()
            )
        })?;
        println!(
            "Removed duplicate Ghostty app config: {}",
            app_config.display()
        );
    }

    match fs::remove_dir(&app_config_dir) {
        Ok(()) => println!(
            "Removed empty Ghostty app config directory: {}",
            app_config_dir.display()
        ),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) if err.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
        Err(err) => {
            return Err(format!(
                "{}: cannot remove Ghostty app config directory: {err}",
                app_config_dir.display()
            ));
        }
    }

    Ok(())
}

fn cleanup_legacy_relay_config(home_dir: &Path, managed_relay_config: &Path) -> Result<()> {
    let legacy_dir = home_dir.join(".relay");
    let legacy_config = legacy_dir.join("config.toml");

    if config_matches_managed(&legacy_config, managed_relay_config)? {
        fs::remove_file(&legacy_config).map_err(|err| {
            format!(
                "{}: cannot remove legacy Relay config: {err}",
                legacy_config.display()
            )
        })?;
        println!("Removed legacy Relay config: {}", legacy_config.display());
    }

    match fs::remove_dir(&legacy_dir) {
        Ok(()) => println!(
            "Removed empty legacy Relay config directory: {}",
            legacy_dir.display()
        ),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) if err.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
        Err(err) => {
            return Err(format!(
                "{}: cannot remove legacy Relay config directory: {err}",
                legacy_dir.display()
            ));
        }
    }

    Ok(())
}

fn config_matches_managed(candidate: &Path, managed_config: &Path) -> Result<bool> {
    let metadata = match fs::symlink_metadata(candidate) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => {
            return Err(format!(
                "{}: cannot inspect candidate managed config: {err}",
                candidate.display()
            ));
        }
    };

    if metadata.file_type().is_symlink() {
        let managed_resolved = managed_config.canonicalize().map_err(|err| {
            format!(
                "{}: cannot resolve managed config: {err}",
                managed_config.display()
            )
        })?;
        return match candidate.canonicalize() {
            Ok(candidate_resolved) => Ok(candidate_resolved == managed_resolved),
            Err(_) => Ok(false),
        };
    }

    if metadata.is_file() {
        let managed_bytes = fs::read(managed_config).map_err(|err| {
            format!(
                "{}: cannot read managed config: {err}",
                managed_config.display()
            )
        })?;
        let candidate_bytes = fs::read(candidate).map_err(|err| {
            format!(
                "{}: cannot read candidate managed config: {err}",
                candidate.display()
            )
        })?;
        return Ok(candidate_bytes == managed_bytes);
    }

    Ok(false)
}

pub(crate) fn repair_codex_config_command(args: &[String]) -> Result<()> {
    let home_dir = parse_home_arg(args)?;
    repair_codex_config(&home_dir)
}

pub(crate) fn check_install_command(args: &[String]) -> Result<()> {
    let (config_root, home_dir) = parse_install_args(args)?;
    let mut errors = managed_install_link_errors(&config_root, &home_dir)?;
    errors.extend(managed_binary_errors(&home_dir)?);

    let codex_check = codex_skill_installation_check(&config_root, &home_dir)?;
    errors.extend(codex_check.errors);

    let claude_check = claude_skill_installation_check(&config_root, &home_dir)?;
    errors.extend(claude_check.errors);

    if errors.is_empty() {
        println!(
            "Managed config installation is in sync for {}.",
            home_dir.display()
        );
        return Ok(());
    }

    let mut output = String::from("Managed config installation drift detected:");
    for error in errors {
        output.push_str("\n- ");
        output.push_str(&error);
    }
    output.push_str(
        "\nRun `cargo run -- install` from the config checkout to converge managed config links.",
    );
    Err(output)
}

pub(crate) fn check_codex_skills_command(args: &[String]) -> Result<()> {
    let (config_root, home_dir) = parse_install_args(args)?;
    let codex_check = codex_skill_installation_check(&config_root, &home_dir)?;
    if codex_check.errors.is_empty() {
        if codex_check.checked_prompt_input {
            println!(
                "Custom Codex skills are installed and visible to Codex skill/prompt surfaces from {}.",
                codex_check.skills_dir.display()
            );
        } else {
            println!(
                "Custom Codex skills are in sync with {}.",
                codex_check.skills_dir.display()
            );
        }
        return Ok(());
    }

    let mut output = String::from("Custom Codex skill installation drift detected:");
    for error in codex_check.errors {
        output.push_str("\n- ");
        output.push_str(&error);
    }
    output.push_str(
        "\nRun `cargo run -- install` from the config checkout to converge ~/.codex/skills.",
    );
    Err(output)
}

pub(crate) fn check_claude_skills_command(args: &[String]) -> Result<()> {
    let (config_root, home_dir) = parse_install_args(args)?;
    let claude_check = claude_skill_installation_check(&config_root, &home_dir)?;
    if claude_check.errors.is_empty() {
        println!(
            "Custom Claude Code skills are in sync with {}.",
            claude_check.skills_dir.display()
        );
        return Ok(());
    }

    let mut output = String::from("Custom Claude Code skill installation drift detected:");
    for error in claude_check.errors {
        output.push_str("\n- ");
        output.push_str(&error);
    }
    output.push_str(
        "\nRun `cargo run -- install` from the config checkout to converge ~/.claude/skills.",
    );
    Err(output)
}

struct CodexSkillInstallationCheck {
    skills_dir: PathBuf,
    checked_prompt_input: bool,
    errors: Vec<String>,
}

fn codex_skill_installation_check(
    config_root: &Path,
    home_dir: &Path,
) -> Result<CodexSkillInstallationCheck> {
    let agents_dir = config_root.join(".agents");
    let skills_dir = agents_dir.join("skills");

    require_dir(&agents_dir, "config agents directory")?;
    require_dir(&skills_dir, "config skills directory")?;
    validate_config_skills(config_root)?;
    let codex_skill_plan = prepare_codex_skill_links(&skills_dir, home_dir)?;
    let mut errors = skill_link_errors(&codex_skill_plan);
    errors.extend(codex_config_errors(home_dir)?);
    let checked_prompt_input = is_current_home(home_dir);
    if checked_prompt_input {
        let custom_skill_names = codex_skill_plan.implicitly_invocable_skill_names()?;
        errors.extend(codex_prompt_input_errors(
            config_root,
            "config checkout",
            &custom_skill_names,
        )?);
        if !same_existing_path(config_root, home_dir) {
            errors.extend(codex_prompt_input_errors(
                home_dir,
                "home directory",
                &custom_skill_names,
            )?);
        }
    }

    Ok(CodexSkillInstallationCheck {
        skills_dir,
        checked_prompt_input,
        errors,
    })
}

struct ClaudeSkillInstallationCheck {
    skills_dir: PathBuf,
    errors: Vec<String>,
}

fn claude_skill_installation_check(
    config_root: &Path,
    home_dir: &Path,
) -> Result<ClaudeSkillInstallationCheck> {
    let agents_dir = config_root.join(".agents");
    let skills_dir = agents_dir.join("skills");

    require_dir(&agents_dir, "config agents directory")?;
    require_dir(&skills_dir, "config skills directory")?;
    validate_config_skills(config_root)?;
    let claude_skill_plan = prepare_claude_skill_links(&skills_dir, home_dir)?;
    let errors = skill_link_errors(&claude_skill_plan);

    Ok(ClaudeSkillInstallationCheck { skills_dir, errors })
}

fn managed_install_link_errors(config_root: &Path, home_dir: &Path) -> Result<Vec<String>> {
    let managed_links = [
        ManagedLink {
            label: "global agents directory",
            source: config_root.join(".agents"),
            target: home_dir.join(".agents"),
        },
        ManagedLink {
            label: "zsh config directory",
            source: config_root.join("zsh"),
            target: home_dir.join(".zsh"),
        },
        ManagedLink {
            label: "zshrc",
            source: config_root.join("zsh/zshrc"),
            target: home_dir.join(".zshrc"),
        },
        ManagedLink {
            label: "Ghostty config",
            source: config_root.join("ghostty/config"),
            target: home_dir.join(".config/ghostty/config"),
        },
        ManagedLink {
            label: "Relay config",
            source: config_root.join("relay/config.toml"),
            target: home_dir.join(".config/relay/config.toml"),
        },
    ];

    let mut errors = Vec::new();
    for link in managed_links {
        if let Some(error) = managed_link_error(&link)? {
            errors.push(error);
        }
    }
    errors.extend(duplicate_ghostty_app_config_errors(home_dir)?);
    errors.extend(legacy_relay_config_errors(home_dir)?);
    errors.sort();
    Ok(errors)
}

struct ManagedLink {
    label: &'static str,
    source: PathBuf,
    target: PathBuf,
}

fn managed_link_error(link: &ManagedLink) -> Result<Option<String>> {
    if !link.source.exists() {
        return Ok(Some(format!(
            "source for {} is missing: {}",
            link.label,
            link.source.display()
        )));
    }

    let source_resolved = link.source.canonicalize().map_err(|err| {
        format!(
            "{}: cannot resolve source for {}: {err}",
            link.source.display(),
            link.label
        )
    })?;

    match fs::symlink_metadata(&link.target) {
        Ok(metadata) if metadata.file_type().is_symlink() => match link.target.canonicalize() {
            Ok(target_resolved) if target_resolved == source_resolved => Ok(None),
            Ok(target_resolved) => Ok(Some(format!(
                "{} points to {}; expected {}",
                link.target.display(),
                target_resolved.display(),
                source_resolved.display()
            ))),
            Err(err) => Ok(Some(format!(
                "{}: cannot resolve installed {} symlink: {err}",
                link.target.display(),
                link.label
            ))),
        },
        Ok(metadata) if metadata.is_file() => Ok(Some(format!(
            "{} is a regular file; expected symlink to {}",
            link.target.display(),
            source_resolved.display()
        ))),
        Ok(metadata) if metadata.is_dir() => Ok(Some(format!(
            "{} is a directory; expected symlink to {}",
            link.target.display(),
            source_resolved.display()
        ))),
        Ok(_) => Ok(Some(format!(
            "{} exists but is not the expected symlink to {}",
            link.target.display(),
            source_resolved.display()
        ))),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Some(format!(
            "{} is missing; expected symlink to {}",
            link.target.display(),
            source_resolved.display()
        ))),
        Err(err) => Err(format!(
            "{}: cannot inspect installed {}: {err}",
            link.target.display(),
            link.label
        )),
    }
}

fn duplicate_ghostty_app_config_errors(home_dir: &Path) -> Result<Vec<String>> {
    let app_config = home_dir.join("Library/Application Support/com.mitchellh.ghostty/config");
    match fs::symlink_metadata(&app_config) {
        Ok(_) => Ok(vec![format!(
            "{} remains; Ghostty loads it after ~/.config/ghostty/config, so remove it after backing up anything intentional",
            app_config.display()
        )]),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(format!(
            "{}: cannot inspect duplicate Ghostty app config: {err}",
            app_config.display()
        )),
    }
}

fn legacy_relay_config_errors(home_dir: &Path) -> Result<Vec<String>> {
    let legacy_dir = home_dir.join(".relay");
    match fs::symlink_metadata(&legacy_dir) {
        Ok(_) => Ok(vec![format!(
            "{} remains but Relay now reads ~/.config/relay; remove it after backing up anything intentional",
            legacy_dir.display()
        )]),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(format!(
            "{}: cannot inspect legacy Relay config directory: {err}",
            legacy_dir.display()
        )),
    }
}

fn managed_binary_errors(home_dir: &Path) -> Result<Vec<String>> {
    let mut errors = Vec::new();
    errors.extend(config_tools_binary_errors(home_dir)?);
    errors.extend(codex_launcher_errors(home_dir)?);
    errors.sort();
    Ok(errors)
}

fn config_tools_binary_errors(home_dir: &Path) -> Result<Vec<String>> {
    let target = home_dir.join(".local/bin/config-tools");
    match fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.is_file() => {
            if !is_executable_file(&metadata) {
                return Ok(vec![format!("{} is not executable", target.display())]);
            }
            let bytes = fs::read(&target).map_err(|err| {
                format!(
                    "{}: cannot read config-tools binary: {err}",
                    target.display()
                )
            })?;
            if is_probably_config_tools_binary(&bytes) {
                Ok(Vec::new())
            } else {
                Ok(vec![format!(
                    "{} does not look like the managed config-tools binary",
                    target.display()
                )])
            }
        }
        Ok(metadata) if metadata.file_type().is_symlink() => Ok(vec![format!(
            "{} is a symlink; expected managed config-tools executable file",
            target.display()
        )]),
        Ok(_) => Ok(vec![format!(
            "{} exists but is not the managed config-tools executable file",
            target.display()
        )]),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(vec![format!(
            "{} is missing; expected managed config-tools executable file",
            target.display()
        )]),
        Err(err) => Err(format!(
            "{}: cannot inspect config-tools binary: {err}",
            target.display()
        )),
    }
}

fn codex_launcher_errors(home_dir: &Path) -> Result<Vec<String>> {
    let target = home_dir.join(".local/bin/codex");
    match fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.is_file() => {
            if !is_executable_file(&metadata) {
                return Ok(vec![format!("{} is not executable", target.display())]);
            }
            let existing = fs::read_to_string(&target).map_err(|err| {
                format!(
                    "{}: cannot read managed Codex launcher: {err}",
                    target.display()
                )
            })?;
            if existing == codex_launcher_script() {
                Ok(Vec::new())
            } else {
                Ok(vec![format!(
                    "{} does not match the managed Codex launcher",
                    target.display()
                )])
            }
        }
        Ok(metadata) if metadata.file_type().is_symlink() => Ok(vec![format!(
            "{} is a symlink; expected managed Codex launcher file",
            target.display()
        )]),
        Ok(_) => Ok(vec![format!(
            "{} exists but is not the managed Codex launcher file",
            target.display()
        )]),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(vec![format!(
            "{} is missing; expected managed Codex launcher file",
            target.display()
        )]),
        Err(err) => Err(format!(
            "{}: cannot inspect managed Codex launcher: {err}",
            target.display()
        )),
    }
}

fn validate_config_skills(config_root: &Path) -> Result<()> {
    let errors = validate_registry(config_root);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Skill validation failed:\n- {}",
            errors.join("\n- ")
        ))
    }
}

fn install_config_tools_binary(home_dir: &Path) -> Result<()> {
    let source = env::current_exe()
        .map_err(|err| format!("cannot determine config-tools executable: {err}"))?;
    if !source.is_file() {
        return Err(format!(
            "{}: config-tools executable is not a file",
            source.display()
        ));
    }

    let bin_dir = home_dir.join(".local/bin");
    fs::create_dir_all(&bin_dir).map_err(|err| {
        format!(
            "{}: cannot create local binary directory: {err}",
            bin_dir.display()
        )
    })?;
    let target = bin_dir.join("config-tools");
    if target
        .canonicalize()
        .ok()
        .zip(source.canonicalize().ok())
        .is_some_and(|(target, source)| target == source)
    {
        println!(
            "config-tools binary already installed at {}",
            target.display()
        );
        return Ok(());
    }

    verify_config_tools_binary_target(&source, &target)?;
    fs::copy(&source, &target).map_err(|err| {
        format!(
            "cannot install config-tools binary from {} to {}: {err}",
            source.display(),
            target.display()
        )
    })?;
    println!("Installed config-tools binary -> {}", target.display());
    Ok(())
}

fn install_codex_launcher(home_dir: &Path) -> Result<()> {
    let bin_dir = home_dir.join(".local/bin");
    fs::create_dir_all(&bin_dir).map_err(|err| {
        format!(
            "{}: cannot create local binary directory: {err}",
            bin_dir.display()
        )
    })?;

    let target = bin_dir.join("codex");
    verify_codex_launcher_target(&target)?;
    fs::write(&target, codex_launcher_script()).map_err(|err| {
        format!(
            "{}: cannot install managed Codex launcher: {err}",
            target.display()
        )
    })?;
    set_executable_permissions(&target)?;
    println!("Installed managed Codex launcher -> {}", target.display());
    Ok(())
}

fn codex_launcher_script() -> &'static str {
    "#!/bin/sh\n\
set -u\n\
\n\
home_dir=${HOME:-}\n\
if [ -n \"$home_dir\" ] && [ -x \"$home_dir/.local/bin/config-tools\" ]; then\n\
  \"$home_dir/.local/bin/config-tools\" repair-codex-config --home \"$home_dir\" >/dev/null 2>&1 || true\n\
fi\n\
\n\
real_codex=${CODEX_REAL_BINARY:-/opt/homebrew/bin/codex}\n\
if [ ! -x \"$real_codex\" ]; then\n\
  printf '%s\\n' \"codex launcher: real Codex binary not found at $real_codex\" >&2\n\
  exit 127\n\
fi\n\
\n\
exec \"$real_codex\" \"$@\"\n"
}

fn verify_codex_launcher_target(target: &Path) -> Result<()> {
    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.is_file() => {
            let existing = fs::read_to_string(target).map_err(|err| {
                format!("{}: cannot read existing Codex launcher: {err}", target.display())
            })?;
            if existing.contains("repair-codex-config")
                && existing.contains("/opt/homebrew/bin/codex")
            {
                Ok(())
            } else {
                Err(format!(
                    "Error: {} already exists and does not look like the managed Codex launcher.\nBack it up and remove it first, then re-run this command.",
                    target.display()
                ))
            }
        }
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "Error: {} is already a symlink to {}.\nBack it up and remove it first, then re-run this command.",
            target.display(),
            fs::read_link(target)
                .map_err(|err| format!("{}: cannot read symlink: {err}", target.display()))?
                .display()
        )),
        Ok(_) => Err(format!(
            "Error: {} already exists and is not the managed Codex launcher.\nBack it up and remove it first, then re-run this command.",
            target.display()
        )),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "{}: cannot inspect Codex launcher target: {err}",
            target.display()
        )),
    }
}

#[cfg(unix)]
fn set_executable_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(|err| {
        format!(
            "{}: cannot set executable permissions: {err}",
            path.display()
        )
    })
}

#[cfg(not(unix))]
fn set_executable_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

fn verify_config_tools_binary_target(source: &Path, target: &Path) -> Result<()> {
    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.is_file() => {
            let source_bytes = fs::read(source)
                .map_err(|err| format!("{}: cannot read current binary: {err}", source.display()))?;
            let target_bytes = fs::read(target).map_err(|err| {
                format!("{}: cannot read existing config-tools binary: {err}", target.display())
            })?;
            if target_bytes == source_bytes
                || (is_executable_file(&metadata) && is_probably_config_tools_binary(&target_bytes))
            {
                Ok(())
            } else {
                Err(format!(
                    "Error: {} already exists and does not look like a managed config-tools binary.\nBack it up and remove it first, then re-run this command.",
                    target.display()
                ))
            }
        }
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let resolved = target.canonicalize().map_err(|err| {
                format!("{}: cannot resolve existing binary symlink: {err}", target.display())
            })?;
            if resolved == source.canonicalize().map_err(|err| {
                format!("{}: cannot resolve current binary: {err}", source.display())
            })? {
                Ok(())
            } else {
                Err(format!(
                    "Error: {} is already a symlink to {}.\nBack it up and remove it first, then re-run this command.",
                    target.display(),
                    fs::read_link(target)
                        .map_err(|err| format!("{}: cannot read symlink: {err}", target.display()))?
                        .display()
                ))
            }
        }
        Ok(_) => Err(format!(
            "Error: {} already exists and is not a config-tools binary.\nBack it up and remove it first, then re-run this command.",
            target.display()
        )),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "{}: cannot inspect config-tools binary target: {err}",
            target.display()
        )),
    }
}

#[cfg(unix)]
fn is_executable_file(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable_file(_metadata: &fs::Metadata) -> bool {
    true
}

fn is_probably_config_tools_binary(bytes: &[u8]) -> bool {
    bytes
        .windows(b"config-tools".len())
        .any(|window| window == b"config-tools")
}

struct SkillLinkPlan {
    label: &'static str,
    links: Vec<(PathBuf, PathBuf)>,
    stale_managed_links: Vec<PathBuf>,
}

impl SkillLinkPlan {
    fn implicitly_invocable_skill_names(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for (source, _) in &self.links {
            if !skill_allows_implicit_invocation(source)? {
                continue;
            }
            let name = source
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string)
                .ok_or_else(|| {
                    format!("{}: skill directory has no valid name", source.display())
                })?;
            names.push(name);
        }
        Ok(names)
    }
}

fn skill_allows_implicit_invocation(skill_dir: &Path) -> Result<bool> {
    let metadata_path = skill_dir.join("agents/openai.yaml");
    let text = match fs::read_to_string(&metadata_path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(err) => {
            return Err(format!(
                "{}: cannot read skill UI metadata: {err}",
                metadata_path.display()
            ))
        }
    };
    let value: serde_yaml::Value = serde_yaml::from_str(&text)
        .map_err(|err| format!("{}: invalid YAML: {err}", metadata_path.display()))?;
    let Some(policy) = value.get("policy") else {
        return Ok(true);
    };
    let Some(allow) = policy.get("allow_implicit_invocation") else {
        return Ok(true);
    };
    allow.as_bool().ok_or_else(|| {
        format!(
            "{}: policy.allow_implicit_invocation must be a boolean",
            metadata_path.display()
        )
    })
}

fn prepare_codex_skill_links(skills_dir: &Path, home_dir: &Path) -> Result<SkillLinkPlan> {
    let codex_skills_dir = home_dir.join(".codex/skills");
    let system_skills_dir = codex_skills_dir.join(".system");
    require_dir(&system_skills_dir, "Codex system skills directory")?;
    verify_codex_system_skills(&system_skills_dir)?;
    prepare_skill_links(skills_dir, &codex_skills_dir, "Codex")
}

fn prepare_claude_skill_links(skills_dir: &Path, home_dir: &Path) -> Result<SkillLinkPlan> {
    let claude_skills_dir = home_dir.join(".claude/skills");
    prepare_skill_links(skills_dir, &claude_skills_dir, "Claude Code")
}

fn prepare_skill_links(
    skills_dir: &Path,
    target_skills_dir: &Path,
    label: &'static str,
) -> Result<SkillLinkPlan> {
    let links = skill_links(skills_dir, target_skills_dir)?;
    let stale_managed_links = stale_managed_skill_links(target_skills_dir, skills_dir, &links)?;
    for (source, target) in &links {
        verify_skill_target(source, target, label)?;
    }
    Ok(SkillLinkPlan {
        label,
        links,
        stale_managed_links,
    })
}

fn repair_codex_config(home_dir: &Path) -> Result<()> {
    let config_path = home_dir.join(".codex/config.toml");
    let text = match fs::read_to_string(&config_path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(format!(
                "{}: cannot read Codex config: {err}",
                config_path.display()
            ))
        }
    };
    let repaired = remove_disallowed_codex_feature_flags(&text);
    if repaired != text {
        fs::write(&config_path, repaired).map_err(|err| {
            format!(
                "{}: cannot write repaired Codex config: {err}",
                config_path.display()
            )
        })?;
        println!(
            "Removed deprecated/disabled Codex feature flags from {}",
            config_path.display()
        );
    }
    Ok(())
}

fn codex_config_errors(home_dir: &Path) -> Result<Vec<String>> {
    let config_path = home_dir.join(".codex/config.toml");
    let text = match fs::read_to_string(&config_path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(format!(
                "{}: cannot read Codex config: {err}",
                config_path.display()
            ))
        }
    };

    let mut errors = Vec::new();
    let mut in_features = false;
    for (line_number, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_features = trimmed == "[features]";
            continue;
        }
        if !in_features {
            continue;
        }
        if trimmed.starts_with("codex_hooks") {
            errors.push(format!(
                "{}:{}: deprecated [features].codex_hooks must be removed; use [features].hooks",
                config_path.display(),
                line_number + 1
            ));
        }
        if trimmed == "apps = false" {
            errors.push(format!(
                "{}:{}: [features].apps must not be forced off because it hides Codex discovery surfaces",
                config_path.display(),
                line_number + 1
            ));
        }
    }
    Ok(errors)
}

fn remove_disallowed_codex_feature_flags(text: &str) -> String {
    let mut output = Vec::new();
    let mut in_features = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_features = trimmed == "[features]";
        }
        if in_features && (trimmed.starts_with("codex_hooks") || trimmed == "apps = false") {
            continue;
        }
        output.push(line);
    }
    let mut repaired = output.join("\n");
    if text.ends_with('\n') {
        repaired.push('\n');
    }
    repaired
}

fn is_current_home(home_dir: &Path) -> bool {
    let Ok(current_home) = env::var("HOME") else {
        return false;
    };
    same_existing_path(home_dir, Path::new(&current_home))
}

fn same_existing_path(left: &Path, right: &Path) -> bool {
    left.canonicalize()
        .ok()
        .zip(right.canonicalize().ok())
        .is_some_and(|(left, right)| left == right)
}

fn codex_prompt_input_errors(
    cwd: &Path,
    label: &str,
    expected_skill_names: &[String],
) -> Result<Vec<String>> {
    let output = Command::new("codex")
        .args(["debug", "prompt-input", "noop"])
        .current_dir(cwd)
        .output()
        .map_err(|err| format!("cannot run codex debug prompt-input noop from {label}: {err}"))?;
    if !output.status.success() {
        return Ok(vec![format!(
            "codex debug prompt-input noop failed from {label} ({}) with {}\nstdout:\n{}\nstderr:\n{}",
            cwd.display(),
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )]);
    }

    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("cannot parse codex debug prompt-input JSON: {err}"))?;
    let mut text = String::new();
    collect_json_text_fields(&value, &mut text);

    let mut errors = Vec::new();
    for name in expected_skill_names {
        let needle = format!("- {name}:");
        if !text.contains(&needle) {
            errors.push(format!(
                "codex debug prompt-input from {label} ({}) did not include managed skill {name:?}",
                cwd.display()
            ));
        }
    }
    Ok(errors)
}

fn collect_json_text_fields(value: &serde_json::Value, output: &mut String) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                if key == "text" {
                    if let Some(text) = value.as_str() {
                        output.push_str(text);
                        output.push('\n');
                    }
                } else {
                    collect_json_text_fields(value, output);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_json_text_fields(value, output);
            }
        }
        _ => {}
    }
}

fn apply_skill_links(plan: SkillLinkPlan) -> Result<()> {
    let label = plan.label;
    for target in plan.stale_managed_links {
        fs::remove_file(&target).map_err(|err| {
            format!(
                "{}: cannot remove stale managed {label} skill symlink: {err}",
                target.display()
            )
        })?;
        println!(
            "Removed stale managed {label} skill symlink: {}",
            target.display()
        );
    }
    for (source, target) in plan.links {
        link_skill(&source, &target, label)?;
    }
    Ok(())
}

fn skill_link_errors(plan: &SkillLinkPlan) -> Vec<String> {
    let label = plan.label;
    let mut errors = Vec::new();
    for target in &plan.stale_managed_links {
        errors.push(format!(
            "{} is a stale managed {label} skill symlink",
            target.display()
        ));
    }
    for (source, target) in &plan.links {
        match fs::symlink_metadata(target) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                if let Err(err) = verify_skill_target(source, target, label) {
                    errors.push(err);
                }
            }
            Ok(_) => errors.push(format!(
                "{} exists but is not the expected {label} skill symlink to {}",
                target.display(),
                source.display()
            )),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => errors.push(format!(
                "{} is missing; expected symlink to {}",
                target.display(),
                source.display()
            )),
            Err(err) => errors.push(format!(
                "{}: cannot inspect {label} skill target: {err}",
                target.display()
            )),
        }
    }
    errors.sort();
    errors
}

fn skill_links(skills_dir: &Path, target_skills_dir: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    discover_custom_skills(skills_dir)?
        .into_iter()
        .map(|skill_dir| {
            let name = skill_dir
                .file_name()
                .ok_or_else(|| format!("{}: skill directory has no name", skill_dir.display()))?;
            Ok((skill_dir.clone(), target_skills_dir.join(name)))
        })
        .collect()
}

fn stale_managed_skill_links(
    target_skills_dir: &Path,
    skills_dir: &Path,
    active_links: &[(PathBuf, PathBuf)],
) -> Result<Vec<PathBuf>> {
    let active_targets: BTreeSet<PathBuf> = active_links
        .iter()
        .map(|(_, target)| normalize_path(target))
        .collect();
    let managed_skills_dir = normalize_path(skills_dir);
    let mut stale_links = Vec::new();

    let entries = match fs::read_dir(target_skills_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(format!(
                "{}: cannot read skills directory: {err}",
                target_skills_dir.display()
            ))
        }
    };
    for entry in entries {
        let entry = entry.map_err(|err| {
            format!(
                "{}: cannot inspect skills directory entry: {err}",
                target_skills_dir.display()
            )
        })?;
        let target = entry.path();
        if active_targets.contains(&normalize_path(&target)) {
            continue;
        }
        let metadata = fs::symlink_metadata(&target)
            .map_err(|err| format!("{}: cannot inspect skill entry: {err}", target.display()))?;
        if !metadata.file_type().is_symlink() {
            continue;
        }
        let link_target = fs::read_link(&target)
            .map_err(|err| format!("{}: cannot read skill symlink: {err}", target.display()))?;
        let resolved = normalize_existing_ancestor(&resolve_link_target(&target, &link_target)?);
        if resolved.starts_with(&managed_skills_dir) {
            stale_links.push(target);
        }
    }

    stale_links.sort();
    Ok(stale_links)
}

fn resolve_link_target(link_path: &Path, link_target: &Path) -> Result<PathBuf> {
    if link_target.is_absolute() {
        Ok(link_target.to_path_buf())
    } else {
        Ok(link_path
            .parent()
            .ok_or_else(|| format!("{}: symlink has no parent", link_path.display()))?
            .join(link_target))
    }
}

fn normalize_existing_ancestor(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    let mut missing = Vec::new();
    loop {
        if current.exists() {
            if let Ok(mut resolved) = current.canonicalize() {
                for component in missing.iter().rev() {
                    resolved.push(component);
                }
                return normalize_path(&resolved);
            }
        }
        let Some(file_name) = current.file_name().map(|name| name.to_os_string()) else {
            break;
        };
        missing.push(file_name);
        if !current.pop() {
            break;
        }
    }
    normalize_path(path)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::{remove_disallowed_codex_feature_flags, skill_allows_implicit_invocation};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn codex_config_repair_removes_deprecated_and_disabled_feature_flags() {
        let input = "model = \"gpt-5.5\"\n\n[features]\n  codex_hooks = true\n  hooks = true\n  apps = false\n\n[plugins.foo]\nenabled = true\n";
        let expected =
            "model = \"gpt-5.5\"\n\n[features]\n  hooks = true\n\n[plugins.foo]\nenabled = true\n";

        assert_eq!(remove_disallowed_codex_feature_flags(input), expected);
    }

    #[test]
    fn codex_prompt_check_honors_explicit_only_skill_metadata() {
        let skill = tempdir().expect("temporary skill directory");
        assert!(skill_allows_implicit_invocation(skill.path()).expect("missing metadata defaults"));

        let agents_dir = skill.path().join("agents");
        fs::create_dir(&agents_dir).expect("agents directory");
        fs::write(
            agents_dir.join("openai.yaml"),
            "policy:\n  allow_implicit_invocation: false\n",
        )
        .expect("skill metadata");

        assert!(!skill_allows_implicit_invocation(skill.path()).expect("explicit-only metadata"));
    }
}

fn discover_custom_skills(skills_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut skill_dirs = Vec::new();
    for entry in fs::read_dir(skills_dir).map_err(|err| {
        format!(
            "{}: cannot read skills directory: {err}",
            skills_dir.display()
        )
    })? {
        let entry = entry.map_err(|err| {
            format!(
                "{}: cannot inspect skills directory entry: {err}",
                skills_dir.display()
            )
        })?;
        let path = entry.path();
        if !entry
            .file_type()
            .map_err(|err| format!("{}: cannot inspect file type: {err}", path.display()))?
            .is_dir()
        {
            continue;
        }
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        if path.join("SKILL.md").is_file() {
            skill_dirs.push(path);
        }
    }
    skill_dirs.sort();
    Ok(skill_dirs)
}

fn verify_codex_system_skills(system_skills_dir: &Path) -> Result<()> {
    for skill in ["skill-creator", "skill-installer"] {
        let skill_file = system_skills_dir.join(skill).join("SKILL.md");
        if !skill_file.is_file() {
            return Err(format!(
                "Error: required Codex system skill is missing: {}",
                skill_file.display()
            ));
        }
    }
    Ok(())
}

fn verify_skill_target(source: &Path, target: &Path, label: &str) -> Result<()> {
    let source_resolved = source.canonicalize().map_err(|err| {
        format!(
            "{}: cannot resolve {label} skill source: {err}",
            source.display()
        )
    })?;

    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let target_resolved = target.canonicalize().map_err(|err| {
                format!("{}: cannot resolve {label} skill symlink: {err}", target.display())
            })?;
            if target_resolved == source_resolved {
                Ok(())
            } else {
                let existing = fs::read_link(target).map_err(|err| {
                    format!("{}: cannot read {label} skill symlink: {err}", target.display())
                })?;
                Err(format!(
                    "Error: {} is already a symlink to {}\nBack it up and remove it first if you want to replace it.",
                    target.display(),
                    existing.display()
                ))
            }
        }
        Ok(_) => Err(format!(
            "Error: {} already exists and is not the expected {label} skill symlink.\nBack it up and remove it first, then re-run this command.",
            target.display()
        )),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "{}: cannot inspect {label} skill target: {err}",
            target.display()
        )),
    }
}

fn link_skill(source: &Path, target: &Path, label: &str) -> Result<()> {
    if target.exists() || target.is_symlink() {
        println!(
            "{label} skill already linked correctly: {}",
            target.display()
        );
    } else {
        create_skill_symlink(source, target, label)?;
        println!("Linked {label} skill -> {}", source.display());
    }
    Ok(())
}

fn create_skill_symlink(source: &Path, target: &Path, label: &str) -> Result<()> {
    #[cfg(unix)]
    {
        unix_fs::symlink(source, target).map_err(|err| {
            format!(
                "{}: cannot create {label} skill symlink: {err}",
                target.display()
            )
        })
    }
    #[cfg(not(unix))]
    {
        let _ = (source, target, label);
        Err("install requires unix symlink support".to_string())
    }
}

fn parse_install_args(args: &[String]) -> Result<(PathBuf, PathBuf)> {
    let mut config_root: Option<PathBuf> = None;
    let mut home_dir: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--config-root" => {
                index += 1;
                config_root = Some(PathBuf::from(
                    args.get(index).ok_or("--config-root requires a path")?,
                ));
            }
            "--home" => {
                index += 1;
                home_dir = Some(PathBuf::from(
                    args.get(index).ok_or("--home requires a path")?,
                ));
            }
            unknown => return Err(format!("unknown option: {unknown}")),
        }
        index += 1;
    }

    let config_root = match config_root {
        Some(path) => path,
        None => default_config_root()?,
    }
    .canonicalize()
    .map_err(|err| format!("cannot resolve config root: {err}"))?;

    let home_dir = match home_dir {
        Some(path) => path,
        None => PathBuf::from(env::var("HOME").map_err(|_| "HOME is not set".to_string())?),
    };

    Ok((config_root, home_dir))
}

fn parse_home_arg(args: &[String]) -> Result<PathBuf> {
    let mut home_dir: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--home" => {
                index += 1;
                home_dir = Some(PathBuf::from(
                    args.get(index).ok_or("--home requires a path")?,
                ));
            }
            unknown => return Err(format!("unknown option: {unknown}")),
        }
        index += 1;
    }

    Ok(match home_dir {
        Some(path) => path,
        None => PathBuf::from(env::var("HOME").map_err(|_| "HOME is not set".to_string())?),
    })
}
