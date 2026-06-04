use crate::config_root::discover_config_root;
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
    repair_codex_config(&home_dir)?;
    fs::create_dir_all(home_dir.join(".config/ghostty")).map_err(|err| {
        format!(
            "{}: cannot create Ghostty config directory: {err}",
            home_dir.join(".config/ghostty").display()
        )
    })?;
    fs::create_dir_all(home_dir.join(".relay")).map_err(|err| {
        format!(
            "{}: cannot create Relay config directory: {err}",
            home_dir.join(".relay").display()
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
    link_path(
        &relay_dir.join("config.toml"),
        &home_dir.join(".relay/config.toml"),
        "Relay config",
        &config_root,
    )?;
    apply_codex_skill_links(codex_skill_plan)?;
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
        "Relay config points at {}.",
        relay_dir.join("config.toml").display()
    );
    Ok(())
}

pub(crate) fn repair_codex_config_command(args: &[String]) -> Result<()> {
    let home_dir = parse_home_arg(args)?;
    repair_codex_config(&home_dir)
}

pub(crate) fn check_codex_skills_command(args: &[String]) -> Result<()> {
    let (config_root, home_dir) = parse_install_args(args)?;
    let agents_dir = config_root.join(".agents");
    let skills_dir = agents_dir.join("skills");

    require_dir(&agents_dir, "config agents directory")?;
    require_dir(&skills_dir, "config skills directory")?;
    validate_config_skills(&config_root)?;
    let codex_skill_plan = prepare_codex_skill_links(&skills_dir, &home_dir)?;
    let mut errors = codex_skill_link_errors(&codex_skill_plan);
    errors.extend(codex_config_errors(&home_dir)?);
    let checked_prompt_input = is_current_home(&home_dir);
    if checked_prompt_input {
        let custom_skill_names = codex_skill_plan.custom_skill_names()?;
        errors.extend(codex_prompt_input_errors(
            &config_root,
            "config checkout",
            &custom_skill_names,
        )?);
        if !same_existing_path(&config_root, &home_dir) {
            errors.extend(codex_prompt_input_errors(
                &home_dir,
                "home directory",
                &custom_skill_names,
            )?);
        }
    }
    if errors.is_empty() {
        if checked_prompt_input {
            println!(
                "Custom Codex skills are installed and visible to Codex skill/prompt surfaces from {}.",
                skills_dir.display()
            );
        } else {
            println!(
                "Custom Codex skills are in sync with {}.",
                skills_dir.display()
            );
        }
        return Ok(());
    }

    let mut output = String::from("Custom Codex skill installation drift detected:");
    for error in errors {
        output.push_str("\n- ");
        output.push_str(&error);
    }
    output.push_str(
        "\nRun `cargo run -- install` from the config checkout to converge ~/.codex/skills.",
    );
    Err(output)
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

struct CodexSkillLinkPlan {
    links: Vec<(PathBuf, PathBuf)>,
    stale_managed_links: Vec<PathBuf>,
}

impl CodexSkillLinkPlan {
    fn custom_skill_names(&self) -> Result<Vec<String>> {
        self.links
            .iter()
            .map(|(source, _)| {
                source
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_string)
                    .ok_or_else(|| {
                        format!("{}: skill directory has no valid name", source.display())
                    })
            })
            .collect()
    }
}

fn prepare_codex_skill_links(skills_dir: &Path, home_dir: &Path) -> Result<CodexSkillLinkPlan> {
    let codex_skills_dir = home_dir.join(".codex/skills");
    let system_skills_dir = codex_skills_dir.join(".system");
    require_dir(&system_skills_dir, "Codex system skills directory")?;
    verify_codex_system_skills(&system_skills_dir)?;

    let links = codex_skill_links(skills_dir, &codex_skills_dir)?;
    let stale_managed_links =
        stale_managed_codex_skill_links(&codex_skills_dir, skills_dir, &links)?;
    for (source, target) in &links {
        verify_codex_skill_target(source, target)?;
    }
    Ok(CodexSkillLinkPlan {
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

fn apply_codex_skill_links(plan: CodexSkillLinkPlan) -> Result<()> {
    for target in plan.stale_managed_links {
        fs::remove_file(&target).map_err(|err| {
            format!(
                "{}: cannot remove stale managed Codex skill symlink: {err}",
                target.display()
            )
        })?;
        println!(
            "Removed stale managed Codex skill symlink: {}",
            target.display()
        );
    }
    for (source, target) in plan.links {
        link_codex_skill(&source, &target)?;
    }
    Ok(())
}

fn codex_skill_link_errors(plan: &CodexSkillLinkPlan) -> Vec<String> {
    let mut errors = Vec::new();
    for target in &plan.stale_managed_links {
        errors.push(format!(
            "{} is a stale managed Codex skill symlink",
            target.display()
        ));
    }
    for (source, target) in &plan.links {
        match fs::symlink_metadata(target) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                if let Err(err) = verify_codex_skill_target(source, target) {
                    errors.push(err);
                }
            }
            Ok(_) => errors.push(format!(
                "{} exists but is not the expected Codex skill symlink to {}",
                target.display(),
                source.display()
            )),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => errors.push(format!(
                "{} is missing; expected symlink to {}",
                target.display(),
                source.display()
            )),
            Err(err) => errors.push(format!(
                "{}: cannot inspect Codex skill target: {err}",
                target.display()
            )),
        }
    }
    errors.sort();
    errors
}

fn codex_skill_links(
    skills_dir: &Path,
    codex_skills_dir: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    discover_custom_skills(skills_dir)?
        .into_iter()
        .map(|skill_dir| {
            let name = skill_dir
                .file_name()
                .ok_or_else(|| format!("{}: skill directory has no name", skill_dir.display()))?;
            Ok((skill_dir.clone(), codex_skills_dir.join(name)))
        })
        .collect()
}

fn stale_managed_codex_skill_links(
    codex_skills_dir: &Path,
    skills_dir: &Path,
    active_links: &[(PathBuf, PathBuf)],
) -> Result<Vec<PathBuf>> {
    let active_targets: BTreeSet<PathBuf> = active_links
        .iter()
        .map(|(_, target)| normalize_path(target))
        .collect();
    let managed_skills_dir = normalize_path(skills_dir);
    let mut stale_links = Vec::new();

    for entry in fs::read_dir(codex_skills_dir).map_err(|err| {
        format!(
            "{}: cannot read Codex skills directory: {err}",
            codex_skills_dir.display()
        )
    })? {
        let entry = entry.map_err(|err| {
            format!(
                "{}: cannot inspect Codex skills directory entry: {err}",
                codex_skills_dir.display()
            )
        })?;
        let target = entry.path();
        if active_targets.contains(&normalize_path(&target)) {
            continue;
        }
        let metadata = fs::symlink_metadata(&target).map_err(|err| {
            format!(
                "{}: cannot inspect Codex skill entry: {err}",
                target.display()
            )
        })?;
        if !metadata.file_type().is_symlink() {
            continue;
        }
        let link_target = fs::read_link(&target).map_err(|err| {
            format!(
                "{}: cannot read Codex skill symlink: {err}",
                target.display()
            )
        })?;
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
    use super::remove_disallowed_codex_feature_flags;

    #[test]
    fn codex_config_repair_removes_deprecated_and_disabled_feature_flags() {
        let input = "model = \"gpt-5.5\"\n\n[features]\n  codex_hooks = true\n  hooks = true\n  apps = false\n\n[plugins.foo]\nenabled = true\n";
        let expected =
            "model = \"gpt-5.5\"\n\n[features]\n  hooks = true\n\n[plugins.foo]\nenabled = true\n";

        assert_eq!(remove_disallowed_codex_feature_flags(input), expected);
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

fn verify_codex_skill_target(source: &Path, target: &Path) -> Result<()> {
    let source_resolved = source.canonicalize().map_err(|err| {
        format!(
            "{}: cannot resolve Codex skill source: {err}",
            source.display()
        )
    })?;

    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let target_resolved = target.canonicalize().map_err(|err| {
                format!(
                    "{}: cannot resolve Codex skill symlink: {err}",
                    target.display()
                )
            })?;
            if target_resolved == source_resolved {
                Ok(())
            } else {
                let existing = fs::read_link(target).map_err(|err| {
                    format!("{}: cannot read Codex skill symlink: {err}", target.display())
                })?;
                Err(format!(
                    "Error: {} is already a symlink to {}\nBack it up and remove it first if you want to replace it.",
                    target.display(),
                    existing.display()
                ))
            }
        }
        Ok(_) => Err(format!(
            "Error: {} already exists and is not the expected Codex skill symlink.\nBack it up and remove it first, then re-run this command.",
            target.display()
        )),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "{}: cannot inspect Codex skill target: {err}",
            target.display()
        )),
    }
}

fn link_codex_skill(source: &Path, target: &Path) -> Result<()> {
    if target.exists() || target.is_symlink() {
        println!("Codex skill already linked correctly: {}", target.display());
    } else {
        create_codex_skill_symlink(source, target)?;
        println!("Linked Codex skill -> {}", source.display());
    }
    Ok(())
}

fn create_codex_skill_symlink(source: &Path, target: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        unix_fs::symlink(source, target).map_err(|err| {
            format!(
                "{}: cannot create Codex skill symlink: {err}",
                target.display()
            )
        })
    }
    #[cfg(not(unix))]
    {
        let _ = (source, target);
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
        None => discover_config_root(
            &env::current_exe()
                .map_err(|err| format!("cannot determine current executable: {err}"))?,
        )?,
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
