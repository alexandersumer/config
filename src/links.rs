use crate::error::Result;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

pub(crate) fn require_dir(path: &Path, label: &str) -> Result<()> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(format!("Error: {label} is missing: {}", path.display()))
    }
}

pub(crate) fn link_path(
    source: &Path,
    target: &Path,
    label: &str,
    config_root: &Path,
) -> Result<()> {
    if !source.exists() {
        return Err(format!(
            "Error: source for {label} does not exist: {}",
            source.display()
        ));
    }

    let source_resolved = source
        .canonicalize()
        .map_err(|err| format!("{}: cannot resolve source: {err}", source.display()))?;
    if let Ok(target_resolved) = target.canonicalize() {
        if normalize_path(&target_resolved) == normalize_path(&source_resolved) {
            println!("{label} already resolves to managed config.");
            return Ok(());
        }
    }

    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let existing = fs::read_link(target)
                .map_err(|err| format!("{}: cannot read symlink: {err}", target.display()))?;
            let existing_resolved = resolve_link_target(target, &existing)?;
            if existing_resolved == normalize_path(&source_resolved) {
                println!("{label} already linked correctly.");
            } else {
                return Err(format!(
                    "Error: {} is already a symlink to {}\nRemove it first if you want to replace it.",
                    target.display(),
                    existing.display()
                ));
            }
        }
        Ok(metadata) if metadata.is_dir() => {
            if fs::read_dir(target)
                .map_err(|err| format!("{}: cannot inspect directory: {err}", target.display()))?
                .next()
                .is_none()
            {
                fs::remove_dir(target).map_err(|err| {
                    format!("{}: cannot remove empty directory: {err}", target.display())
                })?;
                create_symlink(source, target, config_root)?;
                println!(
                    "Replaced empty {label} directory with symlink -> {}",
                    source.display()
                );
            } else {
                return Err(format!(
                    "Error: {} already exists and is not an empty directory.\nBack it up and remove it first, then re-run this command.",
                    target.display()
                ));
            }
        }
        Ok(metadata) if metadata.is_file() && source_resolved.is_file() => {
            let source_bytes = fs::read(&source_resolved).map_err(|err| {
                format!(
                    "{}: cannot read source file: {err}",
                    source_resolved.display()
                )
            })?;
            let target_bytes = fs::read(target)
                .map_err(|err| format!("{}: cannot read existing file: {err}", target.display()))?;
            if source_bytes != target_bytes {
                return Err(format!(
                    "Error: {} already exists with different contents.\nMove the intended contents into the repo first, then re-run this command.",
                    target.display()
                ));
            }
            fs::remove_file(target).map_err(|err| {
                format!("{}: cannot remove matching file: {err}", target.display())
            })?;
            create_symlink(source, target, config_root)?;
            println!(
                "Replaced matching {label} file with symlink -> {}",
                source.display()
            );
        }
        Ok(_) => {
            return Err(format!(
                "Error: {} already exists and is not an empty directory or matching file.\nBack it up and remove it first, then re-run this command.",
                target.display()
            ));
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            create_symlink(source, target, config_root)?;
            println!("Linked {label} -> {}", source.display());
        }
        Err(err) => {
            return Err(format!(
                "{}: cannot inspect target: {err}",
                target.display()
            ))
        }
    }
    Ok(())
}

fn resolve_link_target(link_path: &Path, link_target: &Path) -> Result<PathBuf> {
    let resolved = if link_target.is_absolute() {
        link_target.to_path_buf()
    } else {
        link_path
            .parent()
            .ok_or_else(|| format!("{}: symlink has no parent", link_path.display()))?
            .join(link_target)
    };
    Ok(normalize_existing_ancestor(&resolved))
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

fn create_symlink(source: &Path, target: &Path, config_root: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let link_source = symlink_source(source, target, config_root);
        unix_fs::symlink(&link_source, target)
            .map_err(|err| format!("{}: cannot create symlink: {err}", target.display()))
    }
    #[cfg(not(unix))]
    {
        let _ = (source, target, config_root);
        Err("install requires unix symlink support".to_string())
    }
}

fn symlink_source(source: &Path, target: &Path, config_root: &Path) -> PathBuf {
    if source.starts_with(config_root) && target.starts_with(config_root) {
        if let Some(target_parent) = target.parent() {
            if let (Ok(source_rel), Ok(parent_rel)) = (
                source.strip_prefix(config_root),
                target_parent.strip_prefix(config_root),
            ) {
                let mut relative = PathBuf::new();
                for component in parent_rel.components() {
                    if matches!(component, std::path::Component::Normal(_)) {
                        relative.push("..");
                    }
                }
                relative.push(source_rel);
                return relative;
            }
        }
    }
    source.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symlink_source_uses_relative_path_inside_config_root() {
        let config_root = Path::new("/config");
        let source = Path::new("/config/zsh/zshrc");
        let target = Path::new("/config/generated/zshrc");

        assert_eq!(
            symlink_source(source, target, config_root),
            PathBuf::from("../zsh/zshrc")
        );
    }

    #[test]
    fn symlink_source_counts_parent_components_for_nested_targets() {
        let config_root = Path::new("/config");
        let source = Path::new("/config/.agents/skills");
        let target = Path::new("/config/a/b/c/prompts");

        assert_eq!(
            symlink_source(source, target, config_root),
            PathBuf::from("../../../.agents/skills")
        );
    }

    #[test]
    fn symlink_source_preserves_absolute_path_outside_config_root() {
        let config_root = Path::new("/config");
        let source = Path::new("/other/.agents/skills");
        let target = Path::new("/config/generated/prompts");

        assert_eq!(symlink_source(source, target, config_root), source);
    }

    #[test]
    fn normalize_existing_ancestor_resolves_existing_prefix_and_keeps_missing_tail() {
        let temp_dir = tempfile::Builder::new()
            .prefix("tmp_config_links_test_")
            .tempdir()
            .expect("create temp dir");
        let existing = temp_dir.path().join("existing");
        fs::create_dir(&existing).expect("create existing dir");

        let normalized = normalize_existing_ancestor(&existing.join("missing/child"));
        let expected = existing
            .canonicalize()
            .expect("canonicalize existing dir")
            .join("missing/child");

        assert_eq!(normalized, expected);
    }
}
