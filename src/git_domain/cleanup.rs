//! Conservative linked-worktree policy. Preview never removes files or registrations.
use super::{discover, display, git, git::diagnostic, lock};
use crate::{presentation, runtime};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{Read, Seek, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::{ffi::OsStringExt, fs::MetadataExt, process::CommandExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, clap::Args)]
pub(crate) struct Args {
    /// Checkout, linked worktree, or container directories (default: current directory)
    #[arg(default_value = ".", value_name="PATH", value_hint=clap::ValueHint::DirPath)]
    paths: Vec<PathBuf>,
    /// Remove idle worktrees and ignored files; save HEAD locally and protect tracked/untracked changes
    #[arg(long)]
    apply: bool,
    /// Authorize loss of tracked changes, untracked files, and ignored files at this exact worktree; repeat per path
    #[arg(long, value_name="PATH", value_hint=clap::ValueHint::DirPath)]
    discard_local: Vec<PathBuf>,
    /// Authorize loss of ignored files only at this exact worktree; tracked and untracked data remain protected
    #[arg(long, value_name="PATH", value_hint=clap::ValueHint::DirPath)]
    discard_ignored: Vec<PathBuf>,
    /// Require remote publication and exact ignored-file approval instead of pragmatic cleanup
    #[arg(long)]
    strict: bool,
    /// Preserve committed history in a local recovery ref at this exact worktree (also available in strict mode)
    #[arg(long, value_name="PATH", value_hint=clap::ValueHint::DirPath)]
    preserve_commits: Vec<PathBuf>,
    /// Remote whose current heads and tags must contain every commit being removed in strict mode
    #[arg(long, default_value="origin", value_parser=super::git_name)]
    remote: String,
    /// Deadline per inspection or removal (1-86400 seconds); cleanup never retries
    #[arg(long, default_value_t=300, value_parser=clap::value_parser!(u32).range(1..=86_400), value_name="SECONDS")]
    timeout: u32,
    /// Show successful Git diagnostic output on stderr
    #[arg(short, long)]
    verbose: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Registration {
    path: PathBuf,
    head: Option<String>,
    locked: bool,
    prunable: bool,
    bare: bool,
}
fn registrations(bytes: &[u8]) -> Result<Vec<Registration>, String> {
    let mut records = Vec::new();
    let mut current = None;
    for field in bytes.split(|b| *b == 0) {
        if field.is_empty() {
            if let Some(r) = current.take() {
                records.push(r);
            }
        } else if let Some(path) = field.strip_prefix(b"worktree ") {
            if current.is_some() {
                return Err("malformed worktree registrations".into());
            }
            current = Some(Registration {
                path: PathBuf::from(OsString::from_vec(path.to_vec())),
                head: None,
                locked: false,
                prunable: false,
                bare: false,
            });
        } else {
            let r = current.as_mut().ok_or("malformed worktree registration")?;
            if let Some(oid) = field.strip_prefix(b"HEAD ") {
                r.head = Some(String::from_utf8(oid.to_vec()).map_err(|_| "invalid HEAD")?);
            } else if field.starts_with(b"locked") {
                r.locked = true;
            } else if field.starts_with(b"prunable") {
                r.prunable = true;
            } else if field == b"bare" {
                r.bare = true;
            } else if field != b"detached" && !field.starts_with(b"branch ") {
                return Err("unsupported worktree registration".into());
            }
        }
    }
    if let Some(r) = current {
        records.push(r);
    }
    if records.is_empty() {
        return Err("no primary worktree registration".into());
    }
    Ok(records)
}
fn list(repo: &Path) -> Result<Vec<Registration>, String> {
    registrations(&git::probe(
        repo,
        &["worktree", "list", "--porcelain", "-z"],
    )?)
}
struct Candidate {
    primary: PathBuf,
    common: PathBuf,
    registration: Registration,
}
struct Selection {
    candidates: Vec<Candidate>,
    protected: BTreeSet<PathBuf>,
}
fn select(paths: &[PathBuf]) -> Result<Selection, String> {
    let mut progress = presentation::Discovery::new("worktrees")?;
    let mut containers = Vec::new();
    let mut explicit = BTreeSet::new();
    let mut explicit_linked = BTreeSet::new();
    let mut repos = BTreeMap::new();
    let mut walked = BTreeSet::new();
    // Validate the entire scope before publication probes or mutation.
    for path in paths {
        progress.update()?;
        let p = path
            .canonicalize()
            .map_err(|e| format!("{}: {e}", display(path)))?;
        if !p.is_dir() {
            return Err(format!("{}: expected directory", display(path)));
        }
        if let Some((repo, linked)) = discover::repository(&p)? {
            if linked {
                explicit_linked.insert(repo.path.clone());
            } else {
                explicit.insert(repo.common.clone());
            }
            repos.insert(repo.common, repo.path);
        } else {
            containers.push(p);
        }
    }
    let mut stack = containers.clone();
    while let Some(p) = stack.pop() {
        progress.update()?;
        if runtime::cancelled() {
            return Err("Discovery interrupted".into());
        }
        if !walked.insert(p.clone()) {
            continue;
        }
        if git::exists(&p.join(".git"))? {
            let (repo, _) = discover::repository(&p)?.ok_or("invalid Git metadata")?;
            repos.insert(repo.common, repo.path);
            continue;
        }
        for e in fs::read_dir(&p).map_err(|e| e.to_string())? {
            let e = e.map_err(|e| e.to_string())?;
            if e.file_name() != ".git" && e.file_type().map_err(|e| e.to_string())?.is_dir() {
                stack.push(e.path());
            }
        }
    }
    let mut candidates = Vec::new();
    let mut protected = BTreeSet::new();
    for (common, probe) in repos {
        progress.update()?;
        let registrations = list(&probe)?;
        let primary = registrations[0].path.clone();
        // Git's first registration is primary, but independently verify it.
        let (repo, linked) =
            discover::repository(&primary)?.ok_or("cannot verify primary checkout")?;
        if linked || repo.path != primary || repo.common != common {
            return Err("unverifiable primary checkout ownership".into());
        }
        protected.insert(primary.clone());
        for registration in registrations.into_iter().skip(1) {
            if explicit.contains(&common)
                || explicit_linked.contains(&registration.path)
                || containers.iter().any(|p| registration.path.starts_with(p))
            {
                candidates.push(Candidate {
                    primary: primary.clone(),
                    common: common.clone(),
                    registration,
                });
            }
        }
    }
    candidates.sort_by(|a, b| a.registration.path.cmp(&b.registration.path));
    Ok(Selection {
        candidates,
        protected,
    })
}
fn config_value(value: &[u8]) -> Result<Vec<u8>, String> {
    let mut quoted = vec![b'"'];
    for &byte in value {
        if byte.is_ascii_control() {
            return Err("control character in remote URL or reference".into());
        }
        if byte == b'"' || byte == b'\\' {
            quoted.push(b'\\');
        }
        quoted.push(byte);
    }
    quoted.push(b'"');
    Ok(quoted)
}

/// Private destructive-command protocol. The worker, not Git, owns the flock.
pub(crate) fn worker(args: &[OsString]) -> u8 {
    let descriptor = |name| {
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .filter(|&fd| fd > 2 && unsafe { libc::fcntl(fd, libc::F_GETFD) } != -1)
    };
    let Some(lock_fd) = descriptor("WORKCTL_CLEANUP_LOCK") else {
        return 2;
    };
    let Some(terminal_fd) = descriptor("WORKCTL_CLEANUP_TERMINAL") else {
        return 2;
    };
    let Some(timeout) = std::env::var("WORKCTL_CLEANUP_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&v| v > 0 && v <= 86_400_000)
    else {
        return 2;
    };
    if lock_fd == terminal_fd
        || unsafe { libc::getpgrp() != libc::getpid() }
        || !matches!(args.len(), 4 | 5)
        || args[0] != "worktree"
        || args[1] != "remove"
        || args[args.len() - 2] != "--"
        || (args.len() == 5 && args[2] != "--force")
    {
        return 2;
    }
    // Set these before any exec: filters, hooks and filesystem monitors may
    // intentionally outlive Git, but cannot retain repository coordination.
    for fd in [lock_fd, terminal_fd] {
        if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } == -1 {
            return 2;
        }
    }
    let lock = unsafe { fs::File::from_raw_fd(lock_fd) };
    let mut terminal = unsafe { fs::File::from_raw_fd(terminal_fd) };
    if !lock.metadata().is_ok_and(|m| m.is_file())
        || !terminal.metadata().is_ok_and(|m| m.is_file())
    {
        return 2;
    }
    let Some(common) = std::env::var_os("WORKCTL_CLEANUP_COMMON") else {
        return 2;
    };
    let matching = lock
        .metadata()
        .ok()
        .zip(fs::metadata(PathBuf::from(common).join("repo-batch.lock")).ok())
        .is_some_and(|(a, b)| a.dev() == b.dev() && a.ino() == b.ino());
    if !matching {
        return 2;
    }
    let deadline = Instant::now() + Duration::from_millis(timeout);
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(_) => return 1,
    };
    let _signals = match runtime::Signals::install() {
        Ok(s) => s,
        Err(_) => return 1,
    };
    let mut command = git::command(&cwd);
    command
        .args(args)
        .stdin(std::process::Stdio::null())
        .process_group(0);
    let code = match command.spawn() {
        Ok(mut child) => {
            let code = loop {
                match child.try_wait() {
                    Ok(Some(status)) => break status.code().unwrap_or(1),
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("Cannot wait for cleanup Git: {e}");
                        break 1;
                    }
                }
                if runtime::cancelled() {
                    break 130;
                }
                if Instant::now() >= deadline {
                    break 124;
                }
                std::thread::sleep(Duration::from_millis(2));
            };
            // Never kill the final lock owner. Even an uninterruptible Git
            // filesystem operation must finish before exclusion is released.
            let mut reported = false;
            while let Err(error) = runtime::stop_group(&mut child) {
                if !reported {
                    eprintln!("Cleanup Git has not stopped; retaining repository lock: {error}");
                    reported = true;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            code
        }
        Err(e) => {
            eprintln!("Cannot launch cleanup Git: {e}");
            1
        }
    };
    // A terminal result means Git and its entire group are already inert.
    if terminal.write_all(&i32::to_ne_bytes(code)).is_err() {
        return 1;
    }
    code as u8
}

struct Session<'a> {
    args: &'a Args,
    deadline: Instant,
    lock_fd: i32,
    common: &'a Path,
}
impl Session<'_> {
    fn call(&self, path: &Path, args: &[&OsStr]) -> Result<Vec<u8>, String> {
        self.call_with_lock(path, args, false)
    }
    fn call_with_lock(
        &self,
        path: &Path,
        args: &[&OsStr],
        inherit_lock: bool,
    ) -> Result<Vec<u8>, String> {
        let remaining = self
            .deadline
            .checked_duration_since(Instant::now())
            .ok_or("Deadline exceeded")?;
        let mut terminal = tempfile::tempfile().map_err(|e| e.to_string())?;
        let mut cmd = if inherit_lock {
            // A dedicated process holds exclusion until the entire destructive
            // group is stopped. Git and its hooks never receive the lock fd.
            let mut worker =
                std::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
            worker
                .arg("--internal-cleanup-worker")
                .current_dir(path)
                .env("WORKCTL_CLEANUP_LOCK", self.lock_fd.to_string())
                .env("WORKCTL_CLEANUP_COMMON", self.common)
                .env("WORKCTL_CLEANUP_TERMINAL", terminal.as_raw_fd().to_string())
                .env(
                    "WORKCTL_CLEANUP_TIMEOUT_MS",
                    remaining.as_millis().to_string(),
                );
            worker
        } else {
            git::command(path)
        };
        cmd.args(args);
        let fd = self.lock_fd;
        let terminal_fd = terminal.as_raw_fd();
        unsafe {
            cmd.pre_exec(move || {
                for descriptor in [fd, terminal_fd] {
                    if libc::fcntl(
                        descriptor,
                        libc::F_SETFD,
                        if inherit_lock { 0 } else { libc::FD_CLOEXEC },
                    ) == -1
                    {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
        let (mut code, out, err) = if inherit_lock {
            runtime::capture_guarded(cmd, remaining)?
        } else {
            runtime::capture(cmd, remaining, true)?
        };
        if inherit_lock {
            // The worker publishes a result only after its Git group is inert.
            let worker_code = code;
            terminal.rewind().map_err(|e| e.to_string())?;
            let mut bytes = Vec::new();
            terminal
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            code = i32::from_ne_bytes(
                bytes
                    .try_into()
                    .map_err(|_| "Cleanup worker exited without a terminal result".to_string())?,
            );
            if code != worker_code {
                return Err("Cleanup worker exited without a matching terminal result".into());
            }
            if code == 124 {
                return Err("Deadline exceeded; process group terminated".into());
            }
        }
        if code != 0 {
            return Err(format!(
                "git {} failed (exit {code}): {}",
                args.iter()
                    .map(|arg| diagnostic(arg))
                    .collect::<Vec<_>>()
                    .join(" "),
                diagnostic(OsString::from_vec(err))
            ));
        }
        if self.args.verbose && !err.is_empty() {
            writeln!(std::io::stderr(), "{}", diagnostic(OsString::from_vec(err)))
                .map_err(|e| e.to_string())?;
        }
        Ok(out)
    }
    fn probe(&self, path: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
        self.call(path, &args.iter().map(OsStr::new).collect::<Vec<_>>())
    }
    fn text(&self, path: &Path, args: &[&str]) -> Result<String, String> {
        String::from_utf8(self.probe(path, args)?)
            .map(|s| s.trim_end_matches('\n').to_owned())
            .map_err(|_| "non-UTF-8 Git reference".into())
    }
    fn list(&self, path: &Path) -> Result<Vec<Registration>, String> {
        registrations(&self.probe(path, &["worktree", "list", "--porcelain", "-z"])?)
    }
    fn publication(&self, c: &Candidate, head: &str) -> Result<(), String> {
        let shallow = self.text(&c.primary, &["rev-parse", "--is-shallow-repository"])? == "true";
        let urls = self.text(
            &c.primary,
            &["remote", "get-url", "--all", &self.args.remote],
        )?;
        if urls.lines().count() != 1 {
            return Err("remote must resolve to exactly one URL".into());
        }
        let raw_url = self.text(
            &c.primary,
            &[
                "config",
                "--get-all",
                &format!("remote.{}.url", self.args.remote),
            ],
        )?;
        if raw_url.lines().count() != 1 {
            return Err("remote must configure exactly one URL".into());
        }
        let rewrites = self.probe(
            &c.primary,
            &[
                "config",
                "--null",
                "--get-regexp",
                r"^url\..*\.insteadof$|^remote\..*\.url$",
            ],
        )?;
        let advertised = self
            .probe(
                &c.primary,
                &["ls-remote", "--heads", "--tags", "--", &self.args.remote],
            )
            .map_err(|error| format!("remote {}: {error}", diagnostic(&urls)))?;
        let evidence = tempfile::Builder::new()
            .prefix("workctl-publication-")
            .tempdir()
            .map_err(|e| e.to_string())?;
        let format = self.text(&c.primary, &["rev-parse", "--show-object-format"])?;
        let version = self.text(&c.primary, &["--version"])?;
        let version = version
            .split_whitespace()
            .nth(2)
            .unwrap_or("")
            .split('.')
            .take(2)
            .map(|part| part.parse::<u32>())
            .collect::<Result<Vec<_>, _>>();
        let reftable =
            version.is_ok_and(|parts| parts.len() == 2 && (parts[0], parts[1]) >= (2, 45));
        let object_format = format!("--object-format={format}");
        let mut init = vec!["init", "--bare", "--quiet", "--template=", &object_format];
        if reftable {
            init.push("--ref-format=reftable");
        }
        self.probe(evidence.path(), &init)?;
        let probe = evidence.path().join("case-probe");
        fs::write(&probe, []).map_err(|e| e.to_string())?;
        let case_sensitive = !git::exists(&evidence.path().join("CASE-PROBE"))?;
        fs::remove_file(probe).map_err(|e| e.to_string())?;
        let current_ref_fetch = reftable || case_sensitive;
        if shallow {
            // Preserve Git's boundary knowledge in the evidence store, without deepening the source.
            fs::write(
                evidence.path().join("shallow"),
                fs::read(c.common.join("shallow")).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
        // Objects are read from the source only; no refs/FETCH_HEAD are written there.
        let objects = c.common.join("objects");
        if objects.as_os_str().as_encoded_bytes().contains(&b'\n') {
            return Err("object store path contains newline; cannot verify safely".into());
        }
        use std::io::Write;
        let mut alternates = fs::File::create(evidence.path().join("objects/info/alternates"))
            .map_err(|e| e.to_string())?;
        alternates
            .write_all(objects.as_os_str().as_encoded_bytes())
            .and_then(|_| alternates.write_all(b"\n"))
            .map_err(|e| e.to_string())?;
        // Give every advertised source ref a distinct portable destination name.
        // Only this disposable config changes; the source's ref backend stays intact.
        let default_branch = if shallow {
            let prefix = format!("refs/remotes/{}/", self.args.remote);
            self.text(
                &c.primary,
                &[
                    "for-each-ref",
                    "--format=%(symref)",
                    &format!("{prefix}HEAD"),
                ],
            )?
            .strip_prefix(&prefix)
            .map(|name| format!("refs/heads/{name}"))
        } else {
            None
        };
        let mut preferred_mapping = None;
        let remote_name = evidence
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("invalid evidence remote name")?;
        let mut config = b"\n[remote ".to_vec();
        config.extend(config_value(remote_name.as_bytes())?);
        config.extend(b"]\n\turl = ");
        config.extend(config_value(raw_url.as_bytes())?);
        config.push(b'\n');
        // Preserve source rewrite rules and pass the raw URL, so Git expands
        // it exactly once in the isolated store as it does in the source.
        for record in rewrites
            .split(|byte| *byte == 0)
            .filter(|record| !record.is_empty())
        {
            let (key, value) = record.split_at(
                record
                    .iter()
                    .position(|byte| *byte == b'\n')
                    .ok_or("invalid Git configuration record")?,
            );
            if let Some(base) = key
                .strip_prefix(b"url.")
                .and_then(|key| key.strip_suffix(b".insteadof"))
            {
                config.extend(b"[url ");
                config.extend(config_value(base)?);
                config.extend(b"]\n\tinsteadOf = ");
                config.extend(config_value(&value[1..])?);
                config.push(b'\n');
            }
        }
        config.extend(b"[remote ");
        config.extend(config_value(remote_name.as_bytes())?);
        config.extend(b"]\n");
        for (index, record) in advertised
            .split(|b| *b == b'\n')
            .filter(|r| !r.is_empty())
            .enumerate()
        {
            let separator = record
                .iter()
                .position(|b| *b == b'\t')
                .ok_or("invalid remote advertisement")?;
            let name = &record[separator + 1..];
            if name.ends_with(b"^{}") {
                continue;
            }
            if !name.starts_with(b"refs/heads/") && !name.starts_with(b"refs/tags/") {
                return Err("unexpected advertised reference".into());
            }
            let mut mapping = b"+".to_vec();
            mapping.extend(name);
            if current_ref_fetch {
                mapping.extend(b":refs/remotes/evidence/");
                mapping.extend(
                    name.strip_prefix(b"refs/")
                        .ok_or("invalid advertised reference")?,
                );
            } else {
                mapping.extend(format!(":refs/remotes/evidence/{index:08}").as_bytes());
            }
            if default_branch
                .as_ref()
                .is_some_and(|branch| branch.as_bytes() == name)
            {
                preferred_mapping = Some(OsString::from_vec(mapping.clone()));
            }
            if !current_ref_fetch {
                config.extend(b"\tfetch = ");
                config.extend(config_value(&mapping)?);
                config.push(b'\n');
            }
        }
        if current_ref_fetch {
            config.extend(b"\tfetch = +refs/heads/*:refs/remotes/evidence/heads/*\n\tfetch = +refs/tags/*:refs/remotes/evidence/tags/*\n");
        }
        fs::OpenOptions::new()
            .append(true)
            .open(evidence.path().join("config"))
            .and_then(|mut f| f.write_all(&config))
            .map_err(|e| e.to_string())?;
        let mut fetch = vec![
            OsStr::new("-c"),
            OsStr::new("maintenance.auto=false"),
            OsStr::new("-c"),
            OsStr::new("gc.auto=0"),
            OsStr::new("--git-dir"),
            evidence.path().as_os_str(),
            OsStr::new("fetch"),
            OsStr::new("--quiet"),
            OsStr::new("--prune"),
            OsStr::new("--no-tags"),
            OsStr::new("--filter=tree:0"),
            OsStr::new("--no-write-fetch-head"),
            OsStr::new("--no-recurse-submodules"),
        ];
        if shallow {
            fetch.push(OsStr::new("--unshallow"));
        }
        let walk = [
            "--no-replace-objects",
            "rev-list",
            head,
            "--not",
            "--all",
            "--",
        ];
        if let Some(mapping) = preferred_mapping {
            let mut preferred = fetch.clone();
            preferred.extend([
                OsStr::new("--"),
                OsStr::new(remote_name),
                mapping.as_os_str(),
            ]);
            self.call(&c.primary, &preferred)?;
            if self.text(evidence.path(), &walk)?.is_empty() {
                return Ok(());
            }
            if self.text(evidence.path(), &["rev-parse", "--is-shallow-repository"])? != "true" {
                fetch.retain(|arg| *arg != OsStr::new("--unshallow"));
            }
        }
        fetch.extend([OsStr::new("--"), OsStr::new(remote_name)]);
        // Preserve relative URL resolution by retaining the primary working directory.
        self.call(&c.primary, &fetch)?;
        let tips = self.text(
            evidence.path(),
            &[
                "for-each-ref",
                "--format=%(objectname)",
                "refs/remotes/evidence/",
                "refs/tags/",
            ],
        )?;
        if tips.is_empty() {
            return Err("remote has no published heads or tags".into());
        }
        let unpublished = self.text(evidence.path(), &walk)?;
        if !unpublished.is_empty() {
            if self.text(evidence.path(), &["rev-parse", "--is-shallow-repository"])? == "true" {
                return Err("publication not proven with shallow history; push unpublished work or deepen history before cleanup".into());
            }
            return Err(format!(
                "unpublished commits: {}; push or preserve them before cleanup",
                unpublished.lines().count()
            ));
        }
        Ok(())
    }
    fn local_state(
        &self,
        c: &Candidate,
        discard: bool,
        discard_ignored: bool,
    ) -> Result<(Snapshot, PathBuf), String> {
        let path = &c.registration.path;
        if c.registration.locked {
            return Err("worktree is locked; investigate and unlock explicitly".into());
        }
        if c.registration.prunable || c.registration.bare {
            return Err("missing or unsupported registration; ownership is unverifiable".into());
        }
        if path == &c.primary || c.primary.starts_with(path) || c.common.starts_with(path) {
            return Err("main checkout or Git metadata would be affected".into());
        }
        let metadata =
            fs::symlink_metadata(path).map_err(|e| format!("unverifiable ownership: {e}"))?;
        if !metadata.is_dir() || path.canonicalize().map_err(|e| e.to_string())? != *path {
            return Err("symlink or non-directory worktree; ownership is unverifiable".into());
        }
        let marker = fs::symlink_metadata(path.join(".git")).map_err(|e| e.to_string())?;
        if !marker.is_file() {
            return Err("worktree marker is not an owned regular file".into());
        }
        let common = git::path_output(&self.probe(
            path,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )?)
        .canonicalize()
        .map_err(|e| e.to_string())?;
        let own = git::path_output(&self.probe(path, &["rev-parse", "--absolute-git-dir"])?)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let top = git::path_output(&self.probe(
            path,
            &["rev-parse", "--path-format=absolute", "--show-toplevel"],
        )?)
        .canonicalize()
        .map_err(|e| e.to_string())?;
        if common != c.common
            || own == common
            || !own.starts_with(common.join("worktrees"))
            || top != *path
        {
            return Err("worktree ownership changed or cannot be verified".into());
        }
        let owner = git::path_output(&fs::read(own.join("gitdir")).map_err(|e| e.to_string())?);
        if owner != path.join(".git") {
            return Err("Git registration does not own this worktree marker".into());
        }
        let current = self.list(&c.primary)?;
        if !current.contains(&c.registration) {
            return Err("registration changed during inspection".into());
        }
        let check_operations = || -> Result<(), String> {
            for directory in [&own, &common] {
                for marker in [
                    "index.lock",
                    "HEAD.lock",
                    "packed-refs.lock",
                    "shallow.lock",
                    "config.lock",
                    "MERGE_HEAD",
                    "CHERRY_PICK_HEAD",
                    "REVERT_HEAD",
                    "rebase-merge",
                    "rebase-apply",
                    "BISECT_LOG",
                    "sequencer",
                ] {
                    if git::exists(&directory.join(marker))? {
                        return Err(format!("Git lock or operation in progress: {marker}"));
                    }
                }
            }
            Ok(())
        };
        check_operations()?;
        let status = self.probe(
            path,
            &[
                "--no-optional-locks",
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
                "--ignored=matching",
            ],
        )?;
        let flags = self.probe(path, &["ls-files", "-v", "-z"])?;
        let masked = flags
            .split(|b| *b == 0)
            .any(|e| !e.is_empty() && (e[0].is_ascii_lowercase() || e[0] == b'S'));
        let head = self.text(path, &["rev-parse", "--verify", "HEAD"])?;
        if c.registration.head.as_ref() != Some(&head) {
            return Err("HEAD changed since discovery".into());
        }
        let only_ignored = !masked
            && status
                .split(|b| *b == 0)
                .all(|entry| entry.is_empty() || entry.starts_with(b"!! "));
        if (!status.is_empty() || masked) && !discard && !(discard_ignored && only_ignored) {
            return Err(format!(
                "local data ({}) requires {}{}",
                local_data(&status, &flags),
                if only_ignored {
                    "--discard-ignored PATH (ignored files only) or --discard-local PATH"
                } else {
                    "--discard-local PATH"
                },
                local_examples(&status, &flags)
            ));
        }
        check_operations()?;
        Ok((
            Snapshot {
                head,
                status,
                flags,
                device: metadata.dev(),
                inode: metadata.ino(),
                force: discard,
                recovery_ref: None,
                recovery_saved: false,
            },
            own,
        ))
    }
    fn inspect(
        &self,
        c: &Candidate,
        discard: bool,
        discard_ignored: bool,
    ) -> Result<Snapshot, String> {
        let (before, own) = self.local_state(c, discard, discard_ignored)?;
        let path = &c.registration.path;
        let mut stack = vec![path.clone()];
        let mut scanned = Vec::new();
        while let Some(dir) = stack.pop() {
            if runtime::cancelled() || Instant::now() >= self.deadline {
                return Err("nested repository scan interrupted or timed out".into());
            }
            let metadata = fs::symlink_metadata(&dir).map_err(|e| e.to_string())?;
            if !metadata.is_dir() {
                return Err("directory changed during filesystem inspection".into());
            }
            let stamp = |m: &fs::Metadata| {
                (
                    m.dev(),
                    m.ino(),
                    m.mtime(),
                    m.mtime_nsec(),
                    m.ctime(),
                    m.ctime_nsec(),
                )
            };
            scanned.push((dir.clone(), stamp(&metadata)));
            for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.file_name() == ".git" {
                    if dir == *path {
                        continue;
                    } else {
                        return Err(format!("unsupported nested repository: {}", display(&dir)));
                    }
                }
                if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                    let child = entry.path();
                    if child.join("HEAD").is_file() && child.join("objects").is_dir() {
                        return Err(format!(
                            "unsupported nested bare repository: {}",
                            display(&child)
                        ));
                    }
                    stack.push(child);
                }
            }
        }
        let tree = self.probe(path, &["ls-files", "--stage", "-z"])?;
        for entry in tree
            .split(|b| *b == 0)
            .filter(|e| e.starts_with(b"160000 "))
        {
            let separator = entry
                .iter()
                .position(|b| *b == b'\t')
                .ok_or("invalid gitlink entry")?;
            let name = &entry[separator + 1..];
            let submodule = path.join(OsString::from_vec(name.to_vec()));
            match fs::symlink_metadata(&submodule) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Ok(m)
                    if m.is_dir()
                        && fs::read_dir(&submodule)
                            .map_err(|e| e.to_string())?
                            .next()
                            .is_none() => {}
                _ => {
                    return Err(format!(
                        "populated or unverifiable submodule path is protected: {}",
                        display(&submodule)
                    ))
                }
            }
        }
        // Removing the registration can also remove its private module object stores.
        if git::exists(&own.join("modules"))? {
            return Err("submodule repository data in worktree metadata is protected".into());
        }
        // A large ignored directory scan can outlast a commit, an index change,
        // or a new Git operation. Only return freshly revalidated local state.
        let (after, _) = self.local_state(c, discard, discard_ignored)?;
        if before != after {
            return Err("worktree changed during filesystem inspection; refusing removal".into());
        }
        // The final Git probes can invoke hooks or filters. Preserve nested
        // repository data created after its parent directory was scanned.
        for (directory, before) in scanned {
            if runtime::cancelled() || Instant::now() >= self.deadline {
                return Err("filesystem revalidation interrupted or timed out".into());
            }
            let m = fs::symlink_metadata(directory).map_err(|e| e.to_string())?;
            if !m.is_dir()
                || before
                    != (
                        m.dev(),
                        m.ino(),
                        m.mtime(),
                        m.mtime_nsec(),
                        m.ctime(),
                        m.ctime_nsec(),
                    )
            {
                return Err(
                    "directory changed during filesystem inspection; refusing removal".into(),
                );
            }
        }
        if git::exists(&own.join("modules"))? {
            return Err("submodule repository data in worktree metadata is protected".into());
        }
        Ok(after)
    }
}
#[derive(PartialEq, Eq)]
struct Snapshot {
    head: String,
    status: Vec<u8>,
    flags: Vec<u8>,
    device: u64,
    inode: u64,
    force: bool,
    recovery_ref: Option<String>,
    recovery_saved: bool,
}

fn local_data(status: &[u8], flags: &[u8]) -> String {
    let mut tracked = 0;
    let mut untracked = 0;
    let mut ignored = 0;
    let mut entries = status.split(|b| *b == 0).filter(|e| !e.is_empty());
    while let Some(entry) = entries.next() {
        if entry.starts_with(b"?? ") {
            untracked += 1;
        } else if entry.starts_with(b"!! ") {
            ignored += 1;
        } else {
            tracked += 1;
            if entry
                .get(..2)
                .is_some_and(|s| s.contains(&b'R') || s.contains(&b'C'))
            {
                entries.next();
            }
        }
    }
    let masked = flags
        .split(|b| *b == 0)
        .filter(|e| !e.is_empty() && (e[0].is_ascii_lowercase() || e[0] == b'S'))
        .count();
    format!("{tracked} tracked changes, {untracked} untracked entries, {ignored} ignored entries, {masked} masked tracked paths")
}
fn local_examples(status: &[u8], flags: &[u8]) -> String {
    let mut groups: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut entries = status.split(|b| *b == 0).filter(|e| !e.is_empty());
    while let Some(entry) = entries.next() {
        if entry.len() < 3 {
            continue;
        }
        let kind = if entry.starts_with(b"?? ") {
            "Untracked"
        } else if entry.starts_with(b"!! ") {
            "Ignored"
        } else {
            "Tracked"
        };
        groups
            .entry(kind)
            .or_default()
            .push(display(OsString::from_vec(entry[3..].to_vec())));
        if kind == "Tracked" && entry[..2].iter().any(|b| *b == b'R' || *b == b'C') {
            entries.next();
        }
    }
    for entry in flags
        .split(|b| *b == 0)
        .filter(|e| e.len() > 2 && (e[0].is_ascii_lowercase() || e[0] == b'S'))
    {
        groups
            .entry("Masked")
            .or_default()
            .push(display(OsString::from_vec(entry[2..].to_vec())));
    }
    groups
        .into_iter()
        .map(|(kind, paths)| {
            format!(
                "\n{kind} examples: {}{}",
                paths.iter().take(3).cloned().collect::<Vec<_>>().join(", "),
                if paths.len() > 3 {
                    format!(" (+{} more)", paths.len() - 3)
                } else {
                    String::new()
                }
            )
        })
        .collect()
}

struct Failure {
    status: &'static str,
    reason: String,
    evidence: Option<Snapshot>,
}
impl From<String> for Failure {
    fn from(reason: String) -> Self {
        Self {
            status: "blocked",
            reason,
            evidence: None,
        }
    }
}
impl From<&str> for Failure {
    fn from(reason: &str) -> Self {
        reason.to_owned().into()
    }
}
fn execute(
    args: &Args,
    candidate: &Candidate,
    discard: bool,
    discard_ignored: bool,
    preserve_commits: bool,
) -> Result<Snapshot, Failure> {
    if runtime::cancelled() {
        return Err("Interrupted before inspection".into());
    }
    let held = lock::acquire(&candidate.common)?;
    let session = Session {
        args,
        deadline: Instant::now() + Duration::from_secs(args.timeout.into()),
        lock_fd: held.as_raw_fd(),
        common: &candidate.common,
    };
    let mut first = session.inspect(candidate, discard, discard_ignored)?;
    if !preserve_commits {
        session.publication(candidate, &first.head)?;
    }
    if args.apply {
        let second = session.inspect(candidate, discard, discard_ignored)?;
        if first != second {
            return Err("worktree changed during inspection; refusing removal".into());
        }
        if !preserve_commits {
            session.publication(candidate, &second.head)?;
        }
        // Remote I/O may take time. Finish with local ownership, lock, HEAD and
        // file revalidation so native removal follows the last local inspection.
        let immediate = session.inspect(candidate, discard, discard_ignored)?;
        if second != immediate {
            return Err(
                "worktree changed during publication verification; refusing removal".into(),
            );
        }
        if preserve_commits {
            let recovery = format!("refs/workctl/cleanup/{}", first.head);
            // Atomic create-or-verify: never overwrite an existing recovery ref.
            if !session
                .text(
                    &candidate.primary,
                    &["for-each-ref", "--format=%(symref)", &recovery],
                )?
                .trim()
                .is_empty()
            {
                return Err("recovery ref is symbolic; refusing removal".into());
            }
            let old = session.text(&candidate.primary, &["rev-parse", "--verify", &recovery]);
            if old.as_ref().is_ok_and(|head| head != &first.head) {
                return Err("recovery ref points at different history; refusing removal".into());
            }
            if old.is_err() {
                // Creating a recovery pin cannot remove worktree data. Keep
                // hooks and their vetoes, but don't let a detached hook service
                // inherit our flock. Destructive commands still inherit it so
                // killing the supervisor cannot release it before removal ends.
                let zero = "0".repeat(first.head.len());
                session.call_with_lock(
                    &candidate.primary,
                    &["update-ref", "--no-deref", &recovery, &first.head, &zero].map(OsStr::new),
                    false,
                )?;
            }
            if session.text(&candidate.primary, &["rev-parse", "--verify", &recovery])?
                != first.head
            {
                return Err("recovery ref did not verify; refusing removal".into());
            }
            first.recovery_ref = Some(recovery);
            first.recovery_saved = true;
            // Saving a ref is another subprocess boundary. Recheck local state
            // afterwards so a commit or edit made during recovery cannot be lost.
            let final_state = match session.inspect(candidate, discard, discard_ignored) {
                Ok(state) => state,
                Err(reason) => {
                    return Err(Failure {
                        status: "blocked",
                        reason: format!("{reason}; recovery ref retained"),
                        evidence: Some(first),
                    })
                }
            };
            if final_state != immediate {
                return Err(Failure {
                    status: "blocked",
                    reason: "worktree changed while saving recovery history; refusing removal (recovery ref retained)".into(),
                    evidence: Some(first),
                });
            }
        }
        let path = &candidate.registration.path;
        let mut remove = vec![OsStr::new("worktree"), OsStr::new("remove")];
        if immediate.force {
            remove.push(OsStr::new("--force"));
        }
        remove.push(OsStr::new("--"));
        remove.push(path.as_os_str());
        let removal = session.call_with_lock(&candidate.primary, &remove, true);
        let verification = git::exists(path).and_then(|exists| {
            session
                .list(&candidate.primary)
                .map(|registrations| !exists && !registrations.iter().any(|r| r.path == *path))
        });
        let (status, reason) = match (removal, verification) {
            (Ok(_), Ok(true)) => return Ok(first),
            (Err(e), Ok(true)) => (
                "removed",
                format!("removal verified despite Git error: {e}"),
            ),
            (result, Ok(false)) => (
                "failed",
                format!(
                    "removal did not verify: path or Git registration remains{}",
                    result.err().map(|e| format!("; {e}")).unwrap_or_default()
                ),
            ),
            (result, Err(e)) => (
                "unverified",
                format!(
                    "removal outcome could not be verified: {e}{}",
                    result.err().map(|e| format!("; {e}")).unwrap_or_default()
                ),
            ),
        };
        return Err(Failure {
            status,
            reason,
            evidence: Some(first),
        });
    }
    if preserve_commits {
        first.recovery_ref = Some(format!("refs/workctl/cleanup/{}", first.head));
    }
    Ok(first)
}

// ANSI-C quoting works in the supported zsh and bash terminals, including
// paths containing controls or non-UTF-8 bytes. Never render raw controls.
fn shell_arg(value: &OsStr) -> String {
    let bytes = value.as_encoded_bytes();
    if !bytes.is_empty()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_./:=+".contains(byte))
    {
        return String::from_utf8(bytes.to_vec()).expect("ASCII shell argument");
    }
    let mut out = String::from("$'");
    for &byte in bytes {
        match byte {
            b'\'' => out.push_str("\\'"),
            b'\\' => out.push_str("\\\\"),
            32..=126 => out.push(byte as char),
            _ => out.push_str(&format!("\\x{byte:02x}")),
        }
    }
    out.push('\'');
    out
}

fn next_step(
    args: &Args,
    path: &Path,
    status: &str,
    reason: Option<&str>,
    discard: bool,
    discard_ignored: bool,
    json: bool,
) -> Option<serde_json::Value> {
    let local = reason.is_some_and(|reason| reason.starts_with("local data ("));
    let ignored = local && reason.is_some_and(|reason| reason.contains("--discard-ignored PATH"));
    let unpublished = reason.is_some_and(|reason| {
        reason.starts_with("unpublished commits:")
            || reason.starts_with("publication not proven with shallow history")
    });
    let (action, destructive, mut command) = if status == "would remove" || ignored {
        let mut command: Vec<OsString> = ["workctl", "git", "worktree", "clean"]
            .iter()
            .map(OsString::from)
            .collect();
        if status == "would remove" {
            command.push("--apply".into());
        }
        if json {
            command.push("--json".into());
        }
        if args.strict {
            command.push("--strict".into());
        }
        if args
            .preserve_commits
            .iter()
            .any(|approved| approved.canonicalize().ok().as_deref() == Some(path))
        {
            command.extend(["--preserve-commits".into(), path.as_os_str().to_owned()]);
        }
        if args.remote != "origin" {
            command.extend(["--remote".into(), args.remote.clone().into()]);
        }
        if args.timeout != 300 {
            command.extend(["--timeout".into(), args.timeout.to_string().into()]);
        }
        if discard {
            command.extend(["--discard-local".into(), path.as_os_str().to_owned()]);
        } else if ignored || discard_ignored {
            command.extend(["--discard-ignored".into(), path.as_os_str().to_owned()]);
        }
        (
            if ignored {
                "preview_ignored_removal"
            } else {
                "apply_eligible_removal"
            },
            !ignored,
            command,
        )
    } else if local || unpublished {
        let mut command: Vec<OsString> = ["git", "--no-optional-locks", "-C"]
            .iter()
            .map(OsString::from)
            .collect();
        command.push(path.as_os_str().to_owned());
        command.extend(
            if unpublished {
                vec!["log", "--oneline", "--decorate", "-20"]
            } else {
                vec![
                    "status",
                    "--short",
                    "--untracked-files=all",
                    "--ignored=matching",
                ]
            }
            .into_iter()
            .map(OsString::from),
        );
        return Some(
            serde_json::json!({"action":if unpublished {"inspect_commits"} else {"inspect_local_data"},"command":command.iter().map(|arg|shell_arg(arg)).collect::<Vec<_>>().join(" "),"destructive":false}),
        );
    } else {
        return None;
    };
    command.push(path.as_os_str().to_owned());
    Some(
        serde_json::json!({"action":action,"command":command.iter().map(|arg|shell_arg(arg)).collect::<Vec<_>>().join(" "),"destructive":destructive}),
    )
}

pub(crate) fn run(args: Args, json: bool) -> u8 {
    let _signals = match runtime::Signals::install() {
        Ok(s) => s,
        Err(e) => {
            super::report(&e);
            return 1;
        }
    };
    match run_inner(&args, json) {
        Ok(code) => code,
        Err(e) => {
            super::report(&e);
            let code = if runtime::interrupted() { 130 } else { 1 };
            if json {
                let _ = presentation::json(
                    &serde_json::json!({"schema_version":2,"operation":"git.worktree.clean","status":if code==130 {"interrupted"}else{"failed"},"error":e}),
                );
            }
            code
        }
    }
}
fn run_inner(args: &Args, json: bool) -> Result<u8, String> {
    use std::io::Write;
    let started = Instant::now();
    let selected = select(&args.paths)?;
    let mut discard = BTreeSet::new();
    let mut discard_ignored = BTreeSet::new();
    let mut preserve_commits = BTreeSet::new();
    for (option, paths, approved) in [
        ("--discard-local", &args.discard_local, &mut discard),
        (
            "--preserve-commits",
            &args.preserve_commits,
            &mut preserve_commits,
        ),
        (
            "--discard-ignored",
            &args.discard_ignored,
            &mut discard_ignored,
        ),
    ] {
        for p in paths {
            let p = p
                .canonicalize()
                .map_err(|e| format!("{option} path {}: {e}", display(p)))?;
            if !selected.candidates.iter().any(|c| c.registration.path == p) {
                return Err(format!(
                    "{option} path is not a selected linked worktree: {}",
                    display(p)
                ));
            }
            approved.insert(p);
        }
    }
    if !json || !selected.candidates.is_empty() {
        let mut overview: Box<dyn Write> = if json {
            Box::new(std::io::stderr())
        } else {
            Box::new(std::io::stdout())
        };
        writeln!(overview,"Git worktree cleanup{}\n\nScope          {}\nWorktrees      {}\nProtected      {} main checkouts\n",if args.apply {""}else{" preview"},args.paths.iter().map(|p| super::output::short(p)).collect::<Vec<_>>().join(" "),selected.candidates.len(),selected.protected.len()).map_err(|e|e.to_string())?;
        overview.flush().map_err(|e| e.to_string())?;
        drop(overview);
    }
    if !selected.candidates.is_empty() {
        writeln!(
            std::io::stderr(),
            "{} worktrees…",
            if args.apply {
                "Revalidating and removing"
            } else {
                "Inspecting"
            }
        )
        .map_err(|e| e.to_string())?;
    }
    let mut progress = presentation::Progress::new(selected.candidates.len());
    let mut results = Vec::new();
    for candidate in &selected.candidates {
        let path = &candidate.registration.path;
        let action = std::thread::scope(|scope| -> Result<Snapshot, Failure> {
            let (tx, rx) = std::sync::mpsc::channel();
            let discard = discard.contains(path);
            let discard_ignored = !args.strict || discard_ignored.contains(path);
            let preserve_commits = !args.strict || preserve_commits.contains(path);
            scope.spawn(move || {
                let _ = tx.send(execute(
                    args,
                    candidate,
                    discard,
                    discard_ignored,
                    preserve_commits,
                ));
            });
            loop {
                if let Err(e) = progress.update(results.len(), 1) {
                    runtime::cancel();
                    return Err(Failure {
                        status: "unverified",
                        reason: e,
                        evidence: None,
                    });
                }
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(result) => return result,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(e) => {
                        runtime::cancel();
                        return Err(Failure {
                            status: "unverified",
                            reason: format!("cleanup worker reporting failed: {e}"),
                            evidence: None,
                        });
                    }
                }
            }
        });
        let metadata = action.as_ref().map(Some).unwrap_or_else(|e| e.evidence.as_ref()).map(|snapshot| {
            let restore = snapshot.recovery_ref.as_ref().map(|recovery| {
                [OsString::from("git"), OsString::from("-C"), candidate.primary.as_os_str().to_owned(), OsString::from("worktree"), OsString::from("add"), OsString::from("--detach"), path.as_os_str().to_owned(), OsString::from(recovery)]
                    .iter().map(|arg| shell_arg(arg)).collect::<Vec<_>>().join(" ")
            });
            serde_json::json!({
                "head":snapshot.head,
                "local_data":local_data(&snapshot.status,&snapshot.flags),
                "remote":args.remote,
                "publication":if snapshot.recovery_ref.is_some(){"not_checked_local_recovery"}else{"verified"},
                "recovery_ref":snapshot.recovery_ref,
                "recovery_saved":snapshot.recovery_saved,
                "restore_command":restore
            })
        });
        let (status, reason) = match action {
            Ok(_) => (
                if args.apply {
                    "removed"
                } else {
                    "would remove"
                },
                None,
            ),
            Err(e) => {
                let protected =
                    !args.strict && e.status == "blocked" && e.reason.starts_with("local data (");
                (
                    if runtime::cancelled() && e.status == "blocked" {
                        "interrupted"
                    } else if protected {
                        "protected"
                    } else {
                        e.status
                    },
                    Some(if protected {
                        e.reason.replacen(
                            " requires --discard-local PATH",
                            "; local work left in place",
                            1,
                        )
                    } else {
                        e.reason
                    }),
                )
            }
        };
        let next = next_step(
            args,
            path,
            status,
            reason.as_deref(),
            discard.contains(path),
            discard_ignored.contains(path),
            json,
        );
        let mut detail = format!(
            "  {}: {}{}",
            presentation::status(status, reason.is_some() && status != "protected", json),
            display(path),
            reason
                .as_ref()
                .map(|e| e
                    .lines()
                    .map(|line| format!("\n    {}", display(line)))
                    .collect::<String>())
                .unwrap_or_default()
        );
        if let Some(evidence) = &metadata {
            if let Some(recovery) = evidence["recovery_ref"].as_str() {
                detail.push_str(&format!(
                    "\n    {}: {recovery}\n    Local files: {}",
                    if args.apply {
                        "Recovery ref saved"
                    } else {
                        "Recovery ref planned"
                    },
                    evidence["local_data"].as_str().unwrap_or("unknown")
                ));
            }
        }
        if args.verbose {
            if let Some(restore) = metadata
                .as_ref()
                .and_then(|e| e["restore_command"].as_str())
            {
                detail.push_str(&format!("\n    Restore committed files: {restore}"));
            }
        }
        if let Some(next) = &next {
            detail.push_str(&format!(
                "\n    Next: {}",
                next["command"]
                    .as_str()
                    .ok_or("invalid next-step command")?
            ));
        }
        // Permanent rows preserve full deletion paths and block reasons.
        if json {
            progress.message(&detail)?;
        }
        if !json {
            progress.result(&detail)?;
        }
        results.push(serde_json::json!({"path":path.to_string_lossy(),"path_bytes":presentation::path_bytes(path),"status":status,"reason":reason,"discard_local":discard.contains(path),"discard_ignored":!args.strict || discard_ignored.contains(path),"preserve_commits":!args.strict || preserve_commits.contains(path),"evidence":metadata,"next_step":next}));
        progress.update(results.len(), 0)?;
    }
    progress.finish();
    let removed = results.iter().filter(|r| r["status"] == "removed").count();
    let would_remove = results
        .iter()
        .filter(|r| r["status"] == "would remove")
        .count();
    let protected_worktrees = results
        .iter()
        .filter(|r| r["status"] == "protected")
        .count();
    let blocked = results.iter().filter(|r| r["status"] == "blocked").count();
    let interrupted = results
        .iter()
        .filter(|r| r["status"] == "interrupted")
        .count();
    let failed = results.iter().filter(|r| r["status"] == "failed").count();
    let unverified = results
        .iter()
        .filter(|r| r["status"] == "unverified")
        .count();
    let errors = results
        .iter()
        .filter(|r| {
            matches!(r["status"].as_str(), Some("failed" | "unverified"))
                || (r["status"] == "removed" && !r["reason"].is_null())
        })
        .count();
    let code = if runtime::interrupted() {
        130
    } else {
        u8::from(blocked > 0 || errors > 0 || interrupted > 0)
    };
    let status = if code == 130 {
        "interrupted"
    } else if errors > 0 {
        "failed"
    } else if code != 0 {
        "blocked"
    } else if args.apply {
        "completed"
    } else {
        "planned"
    };
    if json {
        if presentation::json(
            &serde_json::json!({"schema_version":2,"operation":"git.worktree.clean","status":status,"apply":args.apply,"policy":if args.strict {"strict"}else{"pragmatic"},"scope":presentation::paths(&args.paths),"protected":presentation::paths(&selected.protected),"results":results,"summary":{"removed":removed,"would_remove":would_remove,"protected_worktrees":protected_worktrees,"blocked":blocked,"failed":failed,"unverified":unverified,"errors":errors,"interrupted":interrupted,"elapsed_seconds":started.elapsed().as_secs_f64()}}),
        ) != 0
        {
            return Ok(1);
        }
    } else {
        let outcome = if code == 130 {
            "Interrupted"
        } else if errors > 0 {
            "Completed with errors"
        } else if code != 0 {
            "Completed with blockers"
        } else if args.apply {
            "Completed"
        } else {
            "Removal plan"
        };
        let mut out = std::io::stdout().lock();
        writeln!(
            out,
            "{}{}\n  {} {}",
            if selected.candidates.is_empty() {
                ""
            } else {
                "\n"
            },
            presentation::status(outcome, code != 0, false),
            if args.apply { removed } else { would_remove },
            if args.apply {
                "removed"
            } else {
                "would remove"
            }
        )
        .map_err(|e| e.to_string())?;
        if protected_worktrees > 0 {
            writeln!(
                out,
                "  {protected_worktrees} protected (local work left in place)"
            )
            .map_err(|e| e.to_string())?;
        }
        if blocked > 0 || args.strict || errors > 0 {
            writeln!(out, "  {blocked} blocked").map_err(|e| e.to_string())?;
        }
        if errors > 0 {
            writeln!(
                out,
                "  {failed} failed\n  {unverified} unverified\n  {errors} errors"
            )
            .map_err(|e| e.to_string())?;
        }
        if interrupted > 0 {
            writeln!(out, "  {interrupted} interrupted").map_err(|e| e.to_string())?;
        }
        writeln!(
            out,
            "  Elapsed {}",
            presentation::duration(started.elapsed())
        )
        .map_err(|e| e.to_string())?;
        if !args.apply && !selected.candidates.is_empty() {
            writeln!(out, "\nPreview only; nothing removed. Apply rechecks current state and may protect newly changed worktrees.").map_err(|e| e.to_string())?;
        }
        if results.iter().any(|result| {
            result["next_step"]["action"] == "preview_ignored_removal"
                || result["next_step"]["action"] == "inspect_local_data"
        }) {
            writeln!(out, "Tracked changes, untracked files, and masked paths stay protected. Inspect these files before deciding whether to discard them.").map_err(|e| e.to_string())?;
        } else if !args.apply && would_remove > 0 {
            writeln!(out, "Apply removes ignored files and saves committed history locally; tracked changes and untracked files stay protected.")
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(code)
}
