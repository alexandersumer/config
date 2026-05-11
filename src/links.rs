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
    repo_root: &Path,
    skills_dir: &Path,
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

    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let existing = fs::read_link(target)
                .map_err(|err| format!("{}: cannot read symlink: {err}", target.display()))?;
            let existing_resolved = resolve_link_target(target, &existing)?;
            let legacy_prompts_resolved =
                normalize_existing_ancestor(&repo_root.join(".agents/prompts"));
            if existing_resolved == normalize_path(&source_resolved) {
                println!("{label} already linked correctly.");
            } else if existing_resolved == legacy_prompts_resolved
                && normalize_path(&source_resolved)
                    == normalize_path(&skills_dir.canonicalize().map_err(|err| {
                        format!(
                            "{}: cannot resolve skills directory: {err}",
                            skills_dir.display()
                        )
                    })?)
            {
                fs::remove_file(target).map_err(|err| {
                    format!("{}: cannot remove legacy symlink: {err}", target.display())
                })?;
                create_symlink(source, target, repo_root)?;
                println!(
                    "Migrated {label} from legacy prompts -> {}",
                    source.display()
                );
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
                create_symlink(source, target, repo_root)?;
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
        Ok(_) => {
            return Err(format!(
                "Error: {} already exists and is not an empty directory.\nBack it up and remove it first, then re-run this command.",
                target.display()
            ));
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            create_symlink(source, target, repo_root)?;
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

fn create_symlink(source: &Path, target: &Path, repo_root: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let link_source = symlink_source(source, target, repo_root);
        unix_fs::symlink(&link_source, target)
            .map_err(|err| format!("{}: cannot create symlink: {err}", target.display()))
    }
    #[cfg(not(unix))]
    {
        let _ = (source, target, repo_root);
        Err("install requires unix symlink support".to_string())
    }
}

fn symlink_source(source: &Path, target: &Path, repo_root: &Path) -> PathBuf {
    if source.starts_with(repo_root) && target.starts_with(repo_root) {
        if let Some(target_parent) = target.parent() {
            if let (Ok(source_rel), Ok(parent_rel)) = (
                source.strip_prefix(repo_root),
                target_parent.strip_prefix(repo_root),
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
