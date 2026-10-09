use super::{display, git};
use crate::runtime as supervisor;
use std::collections::{BTreeSet, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(super) struct Repo {
    pub path: PathBuf,
    pub common: PathBuf,
}
pub(super) struct Selection {
    pub repos: Vec<Repo>,
    pub excluded: BTreeSet<PathBuf>,
}

pub(super) fn repository(path: &Path) -> Result<Option<(Repo, bool)>, String> {
    // A failed probe is only "not a repo" if no .git marker occurs in ancestors.
    let mut marker = false;
    for ancestor in path.ancestors() {
        if git::exists(&ancestor.join(".git"))? {
            marker = true;
            break;
        }
    }
    let (code, out, err) = git::capture(
        path,
        &[
            OsStr::new("rev-parse"),
            OsStr::new("--path-format=absolute"),
            OsStr::new("--show-toplevel"),
        ],
        true,
    )?;
    if code != 0 {
        if marker {
            return Err(format!(
                "{}: corrupt or inaccessible repository: {}",
                display(path),
                git::redact(std::ffi::OsString::from_vec(err)).trim()
            ));
        }
        // Do not classify arbitrary Git/auth/config failures as an empty folder.
        if !String::from_utf8_lossy(&err).contains("not a git repository") {
            return Err(format!(
                "{}: {}",
                display(path),
                git::redact(std::ffi::OsString::from_vec(err)).trim()
            ));
        }
        return Ok(None);
    }
    let top = git::path_output(&out)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let common = git::path_output(&git::probe(
        path,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?)
    .canonicalize()
    .map_err(|e| e.to_string())?;
    let own = git::path_output(&git::probe(path, &["rev-parse", "--absolute-git-dir"])?)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let linked = own != common;
    Ok(Some((Repo { path: top, common }, linked)))
}

pub(super) fn select(paths: &[PathBuf]) -> Result<Selection, String> {
    let mut progress = crate::presentation::Discovery::new("checkouts")?;
    let mut result = Selection {
        repos: vec![],
        excluded: BTreeSet::new(),
    };
    let mut seen = HashSet::new();
    let mut walked = HashSet::new();
    // Prevalidate every explicit input, including worktrees hidden by overlapping roots.
    let mut roots = vec![];
    for path in paths {
        progress.update()?;
        let root = path
            .canonicalize()
            .map_err(|e| format!("{}: {e}", display(path)))?;
        if !root.is_dir() {
            return Err(format!("{}: expected a directory", display(path)));
        }
        if let Some((repo, linked)) = repository(&root)? {
            if linked {
                return Err(format!(
                    "{}: explicit linked worktree refused; select its primary checkout",
                    display(&root)
                ));
            }
            if seen.insert(repo.path.clone()) {
                result.repos.push(repo);
            }
        } else {
            roots.push(root);
        }
    }
    for root in roots {
        let mut stack = vec![root];
        while let Some(path) = stack.pop() {
            progress.update()?;
            if supervisor::cancelled() {
                return Err("Discovery interrupted".into());
            }
            if !walked.insert(path.clone()) {
                continue;
            }
            if git::exists(&path.join(".git"))? {
                let (repo, linked) = repository(&path)?
                    .ok_or_else(|| format!("{}: invalid .git metadata", display(&path)))?;
                if linked {
                    result.excluded.insert(repo.path);
                } else if seen.insert(repo.path.clone()) {
                    result.repos.push(repo);
                }
                continue;
            }
            let mut children = vec![];
            for entry in fs::read_dir(&path).map_err(|e| format!("{}: {e}", display(&path)))? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.file_name() == ".git" {
                    continue;
                }
                if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                    children.push(entry.path());
                }
            }
            children.sort();
            stack.extend(children.into_iter().rev());
        }
    }
    // Stable ordering independent of duplicated or overlapping argument order.
    result.repos.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(result)
}
