use super::{display, git};
use crate::runtime as supervisor;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(super) struct Repo {
    pub path: PathBuf,
    pub common: PathBuf,
}
pub(super) struct Selection {
    pub repos: Vec<Repo>,
    pub excluded: BTreeSet<PathBuf>,
    pub skipped: Skipped,
}

pub(super) type Skipped = BTreeMap<PathBuf, String>;

pub(super) fn validate_explicit(paths: &[PathBuf], skipped: &Skipped) -> Result<(), String> {
    for path in paths {
        if let Some(orphan) = skipped.keys().find(|orphan| path.starts_with(orphan)) {
            return Err(format!(
                "{}: explicit target is inside skipped orphaned worktree {}; inspect its Git metadata before selecting work inside it",
                display(path), display(orphan),
            ));
        }
    }
    Ok(())
}

pub(super) fn skipped_json(skipped: &Skipped) -> Vec<serde_json::Value> {
    skipped
        .iter()
        .map(|(path, reason)| {
            serde_json::json!({
                "path":path.to_string_lossy(), "path_bytes":crate::presentation::path_bytes(path),
                "reason":reason,
            })
        })
        .collect()
}

pub(super) fn report_skipped(skipped: &Skipped) -> Result<(), String> {
    use std::io::Write;
    let mut out = std::io::stderr().lock();
    for (path, reason) in skipped {
        writeln!(out, "Skipped orphaned worktree: {}\n  {}\n  Files left in place; inspect before recovering or removing this directory.", display(path), display(reason))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Read gitfiles as bytes, without following a replaced marker or blocking on a FIFO.
// Other marker types and malformed files remain Git's responsibility.
pub(super) fn gitfile_target(path: &Path) -> Result<Option<PathBuf>, String> {
    let marker = path.join(".git");
    if !fs::symlink_metadata(&marker)
        .map_err(|e| format!("{}: {e}", display(&marker)))?
        .is_file()
    {
        return Ok(None);
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&marker)
        .map_err(|e| format!("{}: {e}", display(&marker)))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    file.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Ok(None);
    }
    let Some(mut target) = bytes.strip_prefix(b"gitdir: ") else {
        return Ok(None);
    };
    while matches!(target.last(), Some(b'\n' | b'\r')) {
        target = &target[..target.len() - 1];
    }
    if target.is_empty() || target.contains(&0) {
        return Ok(None);
    }
    Ok(Some(path.join(PathBuf::from(
        std::ffi::OsString::from_vec(target.to_vec()),
    ))))
}

fn missing_gitdir(path: &Path) -> Result<Option<PathBuf>, String> {
    let Some(target) = gitfile_target(path)? else {
        return Ok(None);
    };
    match fs::symlink_metadata(&target) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Some(target)),
        Err(e) => Err(format!("{}: {e}", display(target))),
        Ok(_) => Ok(None),
    }
}

// Only a missing registration in a verifiable common repository is skippable.
// A missing primary repository, bad config, or uncertain metadata stays fatal.
pub(super) fn orphan_reason(target: &Path) -> Result<Option<String>, String> {
    if target.file_name().is_none() {
        return Ok(None);
    }
    let Some(registrations) = target.parent() else {
        return Ok(None);
    };
    if registrations.file_name() != Some(OsStr::new("worktrees")) {
        return Ok(None);
    }
    let Some(common) = registrations.parent() else {
        return Ok(None);
    };
    let common = match common.canonicalize() {
        Ok(common) => common,
        Err(_) => return Ok(None),
    };
    let (code, out, _) = git::capture(
        &common,
        &[
            OsStr::new("--git-dir"),
            common.as_os_str(),
            OsStr::new("rev-parse"),
            OsStr::new("--path-format=absolute"),
            OsStr::new("--git-common-dir"),
        ],
        true,
    )?;
    if code != 0
        || git::path_output(&out).canonicalize().ok().as_ref() != Some(&common)
        || git::exists(target)?
    {
        return Ok(None);
    }
    Ok(Some(format!(
        "missing worktree registration: {} (common repository: {})",
        display(target),
        display(common)
    )))
}

pub(super) fn discovered_repository(
    path: &Path,
    skipped: &mut Skipped,
) -> Result<Option<(Repo, bool)>, String> {
    let repo = probe_repository(path, Some(skipped))?;
    if repo.is_none() && !skipped.contains_key(path) {
        return Err(format!("{}: invalid .git metadata", display(path)));
    }
    Ok(repo)
}

pub(super) fn repository(path: &Path) -> Result<Option<(Repo, bool)>, String> {
    probe_repository(path, None)
}

fn probe_repository(
    path: &Path,
    skipped: Option<&mut Skipped>,
) -> Result<Option<(Repo, bool)>, String> {
    // A failed probe is only "not a repo" if no .git marker occurs in ancestors.
    let mut marker = None;
    for ancestor in path.ancestors() {
        if git::exists(&ancestor.join(".git"))? {
            marker = Some(ancestor);
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
        if let Some(marker) = marker {
            if let Some(target) = missing_gitdir(marker)? {
                if code == 128
                    && (err.starts_with(b"fatal: not a git repository")
                        || err.starts_with(b"fatal: gitfile does not point to a valid repository:"))
                    && !supervisor::cancelled()
                    && marker == path
                {
                    if let Some(skipped) = skipped {
                        if let Some(reason) = orphan_reason(&target)? {
                            skipped.insert(path.to_owned(), reason);
                            return Ok(None);
                        }
                    }
                }
                return Err(format!("{}: .git points to missing Git metadata: {}; files left in place; inspect before recovering or removing this directory; {}", display(path), display(target), git::diagnostic(std::ffi::OsString::from_vec(err)).trim()));
            }
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

pub(super) fn select(
    paths: &[PathBuf],
    exclusions: &BTreeSet<PathBuf>,
) -> Result<Selection, String> {
    let mut progress = crate::presentation::Discovery::new("checkouts")?;
    let mut result = Selection {
        repos: vec![],
        excluded: BTreeSet::new(),
        skipped: Skipped::new(),
    };
    let mut seen = HashSet::new();
    let mut walked = HashSet::new();
    // Prevalidate every explicit input, including worktrees hidden by overlapping roots.
    let mut roots = vec![];
    let mut explicit = vec![];
    for path in paths {
        progress.update()?;
        let root = path
            .canonicalize()
            .map_err(|e| format!("{}: {e}", display(path)))?;
        if !root.is_dir() {
            return Err(format!("{}: expected a directory", display(path)));
        }
        if exclusions.iter().any(|p| root.starts_with(p)) {
            continue;
        }
        explicit.push(root.clone());
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
            if exclusions.iter().any(|p| path.starts_with(p)) || !walked.insert(path.clone()) {
                continue;
            }
            if git::exists(&path.join(".git"))? {
                if let Some((repo, linked)) = discovered_repository(&path, &mut result.skipped)? {
                    if linked {
                        result.excluded.insert(repo.path);
                    } else if seen.insert(repo.path.clone()) {
                        result.repos.push(repo);
                    }
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
    validate_explicit(&explicit, &result.skipped)?;
    result.repos.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStrExt;

    #[test]
    fn gitfiles_preserve_non_utf8_paths_and_spaces_and_resolve_relative_paths() {
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join(".git");
        for suffix in [b"\n".as_slice(), b"\r\n", b""] {
            let mut content = b"gitdir: ../common/worktrees/name-\xff ".to_vec();
            content.extend_from_slice(suffix);
            fs::write(&marker, content).unwrap();
            let expected = temp.path().join(PathBuf::from(std::ffi::OsString::from_vec(
                b"../common/worktrees/name-\xff ".to_vec(),
            )));
            assert_eq!(gitfile_target(temp.path()).unwrap(), Some(expected));
        }
        fs::write(&marker, b"gitdir: /absolute/worktrees/name-\xff\n").unwrap();
        assert_eq!(
            gitfile_target(temp.path())
                .unwrap()
                .unwrap()
                .as_os_str()
                .as_bytes(),
            b"/absolute/worktrees/name-\xff"
        );
    }

    #[test]
    fn malformed_large_and_special_gitfiles_cannot_authorize_skipping() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join(".git");
        for bytes in [
            b"invalid".as_slice(),
            b"gitdir: \n",
            b"gitdir: /bad\0path\n",
            &vec![b'x'; 65537],
        ] {
            fs::write(&marker, bytes).unwrap();
            assert!(gitfile_target(temp.path()).unwrap().is_none());
        }
        fs::remove_file(&marker).unwrap();
        symlink("absent", &marker).unwrap();
        assert!(gitfile_target(temp.path()).unwrap().is_none());
        fs::remove_file(&marker).unwrap();
        let name = std::ffi::CString::new(marker.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(gitfile_target(temp.path()).unwrap().is_none());
    }
}
