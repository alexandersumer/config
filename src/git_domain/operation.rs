//! One operation, used by every selected primary checkout.
use super::git::{self, Git};
use super::status::Status;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::fd::FromRawFd;
use std::os::unix::{ffi::OsStringExt, fs::MetadataExt};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct Operation {
    git: Git,
    status: Status,
    status_path: PathBuf,
}
impl Operation {
    fn stage(&mut self, stage: &'static str) -> Result<(), String> {
        self.status.stage = stage.into();
        self.status.write(&self.status_path)?;
        println!("Stage: {stage}");
        Ok(())
    }
    fn verify_head(&self, local_ref: &str, oid: &str) -> Result<(), String> {
        let actual_ref = self.git.optional(&["symbolic-ref", "--quiet", "HEAD"])?;
        if !actual_ref.is_some_and(|actual| {
            actual.strip_suffix(b"\n").unwrap_or(&actual) == local_ref.as_bytes()
        }) {
            return Err(
                "Target branch changed during hook or concurrent operation; refusing reset".into(),
            );
        }
        let actual = self.git.text(&["rev-parse", "HEAD"])?;
        if actual != oid {
            return Err(
                "Target branch tip changed during hook or concurrent operation; refusing reset"
                    .into(),
            );
        }
        Ok(())
    }
    fn clean(&self) -> Result<(), String> {
        for entry in self
            .git
            .call(&["ls-files", "-v", "-z"])?
            .split(|b| *b == 0)
            .filter(|entry| !entry.is_empty())
        {
            if entry[0].is_ascii_lowercase() || entry[0] == b'S' {
                return Err(format!(
                    "Tracked path has assume-unchanged or skip-worktree set: {}; refusing reset",
                    super::display(OsString::from_vec(entry[2..].to_vec()))
                ));
            }
        }
        if !self
            .git
            .call(&[
                "--no-optional-locks",
                "status",
                "--porcelain=v1",
                "--untracked-files=no",
            ])?
            .is_empty()
        {
            return Err("Tracked changes or a dirty submodule: refusing reset".into());
        }
        for marker in [
            "MERGE_HEAD",
            "CHERRY_PICK_HEAD",
            "REVERT_HEAD",
            "rebase-merge",
            "rebase-apply",
            "BISECT_LOG",
            "sequencer",
        ] {
            if git::exists(&self.git.metadata(marker))? {
                return Err(format!(
                    "Git operation in progress ({marker}): refusing reset"
                ));
            }
        }
        Ok(())
    }
    fn tree(&self, target: &str) -> Result<(), String> {
        self.clean()?;
        let ignore_case = filesystem_ignores_case(&self.git.cwd)?;
        let key = |path: &[u8]| {
            if ignore_case {
                String::from_utf8_lossy(path).to_lowercase().into_bytes()
            } else {
                path.to_vec()
            }
        };
        let tracked_bytes = self
            .git
            .call(&["ls-tree", "-r", "--name-only", "-z", target])?;
        let tracked_paths: Vec<_> = tracked_bytes
            .split(|b| *b == 0)
            .filter(|p| !p.is_empty())
            .map(key)
            .collect();
        let tracked: BTreeSet<&[u8]> = tracked_paths.iter().map(Vec::as_slice).collect();
        // Deliberately omit --exclude-standard: ignored paths need protection too.
        for entry in self
            .git
            .call(&["ls-files", "--others", "--directory", "-z"])?
            .split(|b| *b == 0)
            .filter(|p| !p.is_empty())
        {
            if !tree_collision(&tracked, &key(entry)) {
                continue;
            }
            // --directory collapses an entire untracked directory to one entry.
            // Sharing a directory is safe; only overlapping contents collide.
            // Expand just this candidate, keeping unrelated caches inexpensive.
            // Git leaves nested repositories opaque and does not follow symlinks.
            let collision = if let Some(path) = entry.strip_suffix(b"/") {
                if tracked.contains(key(path).as_slice()) {
                    Some(entry.to_vec())
                } else {
                    let pathspec = OsString::from_vec([b":(literal)".as_slice(), path].concat());
                    self.git
                        .call_os(&[
                            OsStr::new("ls-files"),
                            OsStr::new("--others"),
                            OsStr::new("-z"),
                            OsStr::new("--"),
                            &pathspec,
                        ])?
                        .split(|b| *b == 0)
                        .find(|p| !p.is_empty() && tree_collision(&tracked, &key(p)))
                        .map(<[u8]>::to_vec)
                }
            } else {
                Some(entry.to_vec())
            };
            if let Some(path) = collision {
                return Err(format!(
                    "Untracked or ignored path would be overwritten: {}",
                    super::display(OsString::from_vec(path))
                ));
            }
        }
        Ok(())
    }
    fn check_target_refs(&self, targets: &[&str]) -> Result<(), String> {
        for target in targets {
            if self
                .git
                .optional(&["symbolic-ref", "--quiet", target])?
                .is_some()
            {
                return Err(format!(
                    "Target ref is symbolic: {target}; refusing to replace another branch"
                ));
            }
        }
        if self.git.text(&["rev-parse", "--show-ref-format"])? == "reftable" {
            return Ok(());
        }
        let refs = self.git.text(&["for-each-ref", "--format=%(refname)"])?;
        let alias = refs.lines().find(|name| {
            targets
                .iter()
                .any(|target| name != target && name.to_lowercase() == target.to_lowercase())
        });
        let Some(alias) = alias else {
            return Ok(());
        };
        let common = self.git.cwd.join(git::path_output(
            &self.git.call(&["rev-parse", "--git-common-dir"])?,
        ));
        if filesystem_ignores_case(&common)? {
            return Err(format!(
                "Target ref conflicts by case with {alias} on this filesystem; refusing reset without changing refs"
            ));
        }
        Ok(())
    }
    fn reset(&mut self, remote: &str, explicit: Option<&str>) -> Result<(), String> {
        self.stage("preflight")?;
        self.clean()?;
        self.git.call(&["remote", "get-url", remote])?;
        if let Some(branch) = explicit {
            self.git
                .call(&["check-ref-format", &format!("refs/heads/{branch}")])?;
        }
        self.stage("resolve remote target")?;
        let wanted = explicit.map(|b| format!("refs/heads/{b}"));
        let mut args = vec!["ls-remote", "--symref", remote, "HEAD"];
        if let Some(ref wanted) = wanted {
            args.push(wanted);
        }
        let advertised = self.git.text(&args)?;
        let branch = match explicit {
            Some(branch) => branch.to_owned(),
            None => {
                let heads: Vec<_> = advertised
                    .lines()
                    .filter_map(|line| {
                        line.strip_prefix("ref: refs/heads/")
                            .and_then(|s| s.strip_suffix("\tHEAD"))
                    })
                    .collect();
                if heads.len() != 1 {
                    return Err("Remote HEAD does not identify one default branch".into());
                }
                heads[0].to_owned()
            }
        };
        if branch.starts_with('-') {
            return Err("Target branch starts with '-' and cannot be switched safely".into());
        }
        self.git
            .call(&["check-ref-format", &format!("refs/heads/{branch}")])?;
        let full = format!("refs/heads/{branch}");
        let advertised_ref = if explicit.is_some() {
            full.as_str()
        } else {
            "HEAD"
        };
        let oid = advertised
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .find(|(value, name)| !value.starts_with("ref:") && *name == advertised_ref)
            .map(|(oid, _)| oid.to_owned())
            .ok_or("Target branch is not advertised by the remote")?;
        let specs = self
            .git
            .text(&["config", "--get-all", &format!("remote.{remote}.fetch")])?;
        let destination = fetch_destination(&specs, &full)?;
        let local_ref = format!("refs/heads/{branch}");
        self.check_target_refs(&[&destination, &local_ref])?;
        self.status.target = Some(format!("{remote}/{branch} ({})", &oid[..oid.len().min(12)]));
        self.status.write(&self.status_path)?;
        println!("Target: {}", self.status.target.as_deref().unwrap());
        self.stage("fetch")?;
        // A prior fetch is not evidence for this attempt, even if Git exits zero
        // without writing FETCH_HEAD (for example through a wrapper).
        match fs::remove_file(self.git.metadata("FETCH_HEAD")) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("Cannot clear previous FETCH_HEAD: {e}")),
        }
        let refspec = format!("+{full}:{destination}");
        let output = self.git.call(&[
            "-c",
            "maintenance.auto=false",
            "-c",
            "gc.auto=0",
            "fetch",
            "--atomic",
            "--write-fetch-head",
            "--no-tags",
            "--no-prune",
            "--no-prune-tags",
            "--no-recurse-submodules",
            "--refmap=",
            remote,
            &refspec,
        ])?;
        print!("{}", String::from_utf8_lossy(&output));
        // A stale preexisting tracking ref is not proof that this fetch included it.
        let fetch_head = fs::read(self.git.metadata("FETCH_HEAD"))
            .map_err(|e| format!("Cannot inspect FETCH_HEAD: {e}"))?;
        let oid = fetched_tip(&fetch_head, &branch)?;
        let fetched = self.git.text(&["rev-parse", "--verify", &destination])?;
        if fetched != oid {
            return Err(
                "Fetched tracking ref differs from FETCH_HEAD; refusing stale reset".into(),
            );
        }
        self.git
            .call(&["cat-file", "-e", &format!("{oid}^{{commit}}")])?;
        self.status.target = Some(format!("{remote}/{branch} ({})", &oid[..oid.len().min(12)]));
        self.status.write(&self.status_path)?;
        println!(
            "Verified target: {}",
            self.status.target.as_deref().unwrap()
        );
        self.stage("check trees and backup")?;
        self.tree(&oid)?;
        let old = self
            .git
            .optional(&["rev-parse", "--verify", "--quiet", &local_ref])?
            .map(|b| String::from_utf8_lossy(&b).trim().to_owned());
        if let Some(old) = &old {
            self.tree(&local_ref)?;
            if old != &oid {
                let stamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|e| e.to_string())?
                    .as_nanos();
                let backup = format!(
                    "refs/home-reset-backups/{stamp}-{}/{branch}",
                    std::process::id()
                );
                self.git
                    .call(&["update-ref", &backup, old, &"0".repeat(old.len())])?;
                self.status.backups.push(backup.clone());
                self.status.write(&self.status_path)?;
                println!("Backup: {backup}");
                println!("Recover with: git branch recovered-work {backup}");
            }
        }
        self.stage("switch")?;
        if old.is_some() {
            self.git.action(&[
                "switch",
                "--no-recurse-submodules",
                "--no-overwrite-ignore",
                &branch,
            ])?;
        } else {
            self.git.action(&[
                "switch",
                "--no-recurse-submodules",
                "--no-overwrite-ignore",
                "--no-track",
                "--create",
                &branch,
                &oid,
            ])?;
        }
        self.status.state = Some(format!("switched to {branch}"));
        self.status.write(&self.status_path)?;
        println!("State: switched to {branch}");
        self.stage("post-switch safety check")?;
        self.tree(&oid)?;
        self.verify_head(&local_ref, old.as_deref().unwrap_or(&oid))?;
        self.stage("set upstream")?;
        self.git.action(&[
            "branch",
            &format!("--set-upstream-to={destination}"),
            &branch,
        ])?;
        self.stage("reset")?;
        self.git
            .action(&["reset", "--no-recurse-submodules", "--hard", &oid])?;
        self.stage("post-reset verification")?;
        self.clean()?;
        self.verify_head(&local_ref, &oid)?;
        self.status.state = Some(format!("reset completed at {oid}"));
        self.status.write(&self.status_path)?;
        println!("State: reset completed at {oid}");
        Ok(())
    }
}

fn filesystem_ignores_case(directory: &std::path::Path) -> Result<bool, String> {
    let probe = tempfile::Builder::new()
        .prefix(".workctl-case-")
        .tempfile_in(directory)
        .map_err(|e| e.to_string())?;
    let upper = probe.path().with_file_name(
        probe
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_uppercase(),
    );
    match fs::metadata(upper) {
        Ok(metadata) => {
            let original = probe.as_file().metadata().map_err(|e| e.to_string())?;
            Ok(metadata.dev() == original.dev() && metadata.ino() == original.ino())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

fn tree_collision(tracked: &BTreeSet<&[u8]>, entry: &[u8]) -> bool {
    let path = entry.strip_suffix(b"/").unwrap_or(entry);
    let prefix = [path, b"/"].concat();
    let parent_collision = path
        .iter()
        .enumerate()
        .filter(|(_, b)| **b == b'/')
        .map(|(i, _)| &path[..i])
        .chain(std::iter::once(path))
        .any(|p| tracked.contains(p));
    parent_collision
        || tracked
            .range(prefix.as_slice()..)
            .next()
            .is_some_and(|p| p.starts_with(&prefix))
}

fn fetched_tip(fetch_head: &[u8], branch: &str) -> Result<String, String> {
    let text = std::str::from_utf8(fetch_head).map_err(|_| "Invalid FETCH_HEAD text")?;
    let mut lines = text.lines();
    let fields: Vec<_> = lines.next().unwrap_or_default().splitn(3, '\t').collect();
    if lines.next().is_some()
        || fields.len() != 3
        || !fields[2].starts_with(&format!("branch '{branch}' of "))
        || !matches!(fields[0].len(), 40 | 64)
        || !fields[0].bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err(
            "FETCH_HEAD does not identify the selected branch; refusing stale reset".into(),
        );
    }
    Ok(fields[0].to_owned())
}

fn matches(pattern: &str, value: &str) -> Option<String> {
    match pattern.split_once('*') {
        Some((before, after)) => value
            .strip_prefix(before)?
            .strip_suffix(after)
            .map(str::to_owned),
        None if pattern == value => Some(String::new()),
        _ => None,
    }
}
fn fetch_destination(specs: &str, target: &str) -> Result<String, String> {
    let specs: Vec<_> = specs.lines().collect();
    if specs.iter().any(|s| {
        s.strip_prefix('^')
            .is_some_and(|s| matches(s, target).is_some())
    }) {
        return Err("Target branch is excluded by existing fetch configuration; configuration left unchanged".into());
    }
    let mut destinations = BTreeSet::new();
    for spec in specs {
        if spec.starts_with('^') {
            continue;
        }
        if let Some((source, dest)) = spec.trim_start_matches('+').split_once(':') {
            let fixed = dest.split('*').next().unwrap_or(dest);
            if dest.starts_with("refs/heads/")
                || (dest.contains('*') && "refs/heads/".starts_with(fixed))
            {
                return Err("Fetch mapping can replace local branches; refusing reset without changing configuration".into());
            }
            if let Some(middle) = matches(source, target) {
                let dest = dest.replace('*', &middle);
                // Do not follow a custom mapping into local branches or tags.
                if !dest.starts_with("refs/remotes/") {
                    return Err("Target fetch mapping does not use remote-tracking refs; configuration left unchanged".into());
                }
                destinations.insert(dest);
            }
        }
    }
    if destinations.len() != 1 {
        return Err("Target requires one unambiguous remote-tracking fetch mapping; configuration left unchanged".into());
    }
    Ok(destinations.into_iter().next().unwrap())
}

pub(super) fn worker(args: &[OsString]) -> u8 {
    let inherited_lock = std::env::var("RESET_TO_ORIGIN_WORKER_LOCK")
        .ok()
        .and_then(|v| v.parse::<i32>().ok());
    if !inherited_lock.is_some_and(|fd| fd > 2 && unsafe { libc::fcntl(fd, libc::F_GETFD) } != -1) {
        eprintln!("Internal workers require a supervisor-owned repository lock");
        return 2;
    }
    if args.len() != 2 {
        eprintln!("Invalid internal worker arguments");
        return 2;
    }
    let Some(remote) = args[0].to_str() else {
        return 2;
    };
    let Some(branch) = args[1].to_str() else {
        return 2;
    };
    let cwd = match std::env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    // Adopt the inherited descriptor for the worker lifetime and validate that
    // it protects this primary checkout, not a caller-supplied arbitrary file.
    let lock = unsafe { fs::File::from_raw_fd(inherited_lock.unwrap()) };
    let git = match Git::new(cwd) {
        Ok(git) => git,
        Err(e) => {
            eprintln!("Error [preflight]: {e}");
            return 1;
        }
    };
    let matching = lock
        .metadata()
        .ok()
        .zip(fs::metadata(git.directory.join("repo-batch.lock")).ok())
        .is_some_and(|(a, b)| a.dev() == b.dev() && a.ino() == b.ino());
    if !matching {
        eprintln!("Internal worker lock does not protect this primary checkout");
        return 2;
    }
    let Some(status_path) = std::env::var_os("RESET_TO_ORIGIN_WORKER_STATUS") else {
        eprintln!("Internal workers require a supervisor-owned status path");
        return 2;
    };
    let mut op = Operation {
        git,
        status: Status {
            stage: "preflight".into(),
            ..Status::default()
        },
        status_path: PathBuf::from(status_path),
    };
    let result = op.reset(
        remote,
        if branch.is_empty() {
            None
        } else {
            Some(branch)
        },
    );
    if let Err(error) = &result {
        let error = git::redact(error);
        eprintln!("Error [{}]: {error}", op.status.stage);
        op.status.error = Some(format!("Error [{}]: {error}", op.status.stage));
    }
    op.status.complete = true;
    if let Err(error) = op.status.write(&op.status_path) {
        eprintln!("{error}");
        return 1;
    }
    if result.is_ok() {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mappings_obey_exclusions_and_preserve_custom_destinations() {
        assert_eq!(
            fetch_destination("+refs/heads/*:refs/remotes/custom/*", "refs/heads/topic/x").unwrap(),
            "refs/remotes/custom/topic/x"
        );
        assert!(fetch_destination(
            "+refs/heads/*:refs/remotes/origin/*\n^refs/heads/topic/*",
            "refs/heads/topic/x"
        )
        .is_err());
        assert!(fetch_destination("+refs/heads/*:refs/heads/*", "refs/heads/main").is_err());
        assert!(fetch_destination(
            "+refs/heads/main:refs/remotes/a/main\n+refs/heads/main:refs/remotes/b/main",
            "refs/heads/main"
        )
        .is_err());
    }
}
