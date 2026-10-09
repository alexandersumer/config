//! Conservative linked-worktree policy. Preview never removes files or registrations.
use super::{discover, display, git, git::diagnostic, lock};
use crate::{presentation, runtime};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::{ffi::OsStringExt, fs::MetadataExt, process::CommandExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, clap::Args)]
pub(crate) struct Args {
    /// Checkout, linked worktree, or container directories (default: current directory)
    #[arg(default_value = ".", value_name="PATH", value_hint=clap::ValueHint::DirPath)]
    paths: Vec<PathBuf>,
    /// Execute eligible removals without another interactive question
    #[arg(long)]
    apply: bool,
    /// Authorize loss of tracked changes, untracked files, and ignored files at this exact worktree; repeat per path
    #[arg(long, value_name="PATH", value_hint=clap::ValueHint::DirPath)]
    discard_local: Vec<PathBuf>,
    /// Authorize loss of ignored files only at this exact worktree; tracked and untracked data remain protected
    #[arg(long, value_name="PATH", value_hint=clap::ValueHint::DirPath)]
    discard_ignored: Vec<PathBuf>,
    /// Remote whose current heads and tags must contain every commit being removed
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

struct Session<'a> {
    args: &'a Args,
    deadline: Instant,
    lock_fd: i32,
}
impl Session<'_> {
    fn call(&self, path: &Path, args: &[&OsStr]) -> Result<Vec<u8>, String> {
        let remaining = self
            .deadline
            .checked_duration_since(Instant::now())
            .ok_or("Deadline exceeded")?;
        let mut cmd = git::command(path);
        cmd.args(args);
        // A killed supervisor cannot release the common lock while descendants run.
        let fd = self.lock_fd;
        unsafe {
            cmd.pre_exec(move || {
                if libc::fcntl(fd, libc::F_SETFD, 0) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let (code, out, err) = runtime::capture(cmd, remaining, true)?;
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
        let advertised =
            self.probe(&c.primary, &["ls-remote", "--heads", "--tags", "--", &urls])?;
        let evidence = tempfile::Builder::new()
            .prefix("workctl-publication-")
            .tempdir()
            .map_err(|e| e.to_string())?;
        let format = self.text(&c.primary, &["rev-parse", "--show-object-format"])?;
        self.probe(
            evidence.path(),
            &[
                "init",
                "--bare",
                "--quiet",
                "--template=",
                &format!("--object-format={format}"),
            ],
        )?;
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
        let remote_name = evidence
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("invalid evidence remote name")?;
        let mut config = b"\n[remote ".to_vec();
        config.extend(config_value(remote_name.as_bytes())?);
        config.extend(b"]\n\turl = ");
        config.extend(config_value(urls.as_bytes())?);
        config.push(b'\n');
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
            mapping.extend(format!(":refs/remotes/evidence/{index:08}").as_bytes());
            config.extend(b"\tfetch = ");
            config.extend(config_value(&mapping)?);
            config.push(b'\n');
        }
        fs::OpenOptions::new()
            .append(true)
            .open(evidence.path().join("config"))
            .and_then(|mut f| f.write_all(&config))
            .map_err(|e| e.to_string())?;
        let mut fetch = vec![
            OsStr::new("--git-dir"),
            evidence.path().as_os_str(),
            OsStr::new("fetch"),
            OsStr::new("--quiet"),
            OsStr::new("--no-tags"),
            OsStr::new("--no-write-fetch-head"),
            OsStr::new("--no-recurse-submodules"),
        ];
        if shallow {
            fetch.push(OsStr::new("--unshallow"));
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
        let unpublished = self.text(
            evidence.path(),
            &[
                "--no-replace-objects",
                "rev-list",
                head,
                "--not",
                "--all",
                "--",
            ],
        )?;
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
    fn inspect(
        &self,
        c: &Candidate,
        discard: bool,
        discard_ignored: bool,
    ) -> Result<Snapshot, String> {
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
        let mut stack = vec![path.clone()];
        while let Some(dir) = stack.pop() {
            if runtime::cancelled() || Instant::now() >= self.deadline {
                return Err("nested repository scan interrupted or timed out".into());
            }
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
        Ok(Snapshot {
            head,
            status,
            flags,
            device: metadata.dev(),
            inode: metadata.ino(),
            force: discard,
        })
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
) -> Result<Snapshot, Failure> {
    if runtime::cancelled() {
        return Err("Interrupted before inspection".into());
    }
    let held = lock::acquire(&candidate.common)?;
    let session = Session {
        args,
        deadline: Instant::now() + Duration::from_secs(args.timeout.into()),
        lock_fd: held.as_raw_fd(),
    };
    let first = session.inspect(candidate, discard, discard_ignored)?;
    session.publication(candidate, &first.head)?;
    if args.apply {
        let second = session.inspect(candidate, discard, discard_ignored)?;
        if first != second {
            return Err("worktree changed during inspection; refusing removal".into());
        }
        session.publication(candidate, &second.head)?;
        // Remote I/O may take time. Finish with local ownership, lock, HEAD and
        // file revalidation so native removal follows the last local inspection.
        let immediate = session.inspect(candidate, discard, discard_ignored)?;
        if second != immediate {
            return Err(
                "worktree changed during publication verification; refusing removal".into(),
            );
        }
        let path = &candidate.registration.path;
        let mut remove = vec![OsStr::new("worktree"), OsStr::new("remove")];
        if immediate.force {
            remove.push(OsStr::new("--force"));
        }
        remove.push(OsStr::new("--"));
        remove.push(path.as_os_str());
        let removal = session.call(&candidate.primary, &remove);
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
    Ok(first)
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
    for (option, paths, approved) in [
        ("--discard-local", &args.discard_local, &mut discard),
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
            let discard_ignored = discard_ignored.contains(path);
            scope.spawn(move || {
                let _ = tx.send(execute(args, candidate, discard, discard_ignored));
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
        let metadata=action.as_ref().map(Some).unwrap_or_else(|e| e.evidence.as_ref()).map(|snapshot| serde_json::json!({"head":snapshot.head,"local_data":local_data(&snapshot.status,&snapshot.flags),"remote":args.remote}));
        let (status, reason) = match action {
            Ok(_) => (
                if args.apply {
                    "removed"
                } else {
                    "would remove"
                },
                None,
            ),
            Err(e) => (
                if runtime::cancelled() && e.status == "blocked" {
                    "interrupted"
                } else {
                    e.status
                },
                Some(e.reason),
            ),
        };
        let detail = format!(
            "  {}: {}{}",
            presentation::status(status, reason.is_some(), json),
            display(path),
            reason
                .as_ref()
                .map(|e| e
                    .lines()
                    .map(|line| format!("\n    {}", display(line)))
                    .collect::<String>())
                .unwrap_or_default()
        );
        // Permanent rows preserve full deletion paths and block reasons.
        if json {
            progress.message(&detail)?;
        }
        if !json {
            progress.result(&detail)?;
        }
        results.push(serde_json::json!({"path":path.to_string_lossy(),"path_bytes":presentation::path_bytes(path),"status":status,"reason":reason,"discard_local":discard.contains(path),"discard_ignored":discard_ignored.contains(path),"evidence":metadata}));
        progress.update(results.len(), 0)?;
    }
    progress.finish();
    let removed = results.iter().filter(|r| r["status"] == "removed").count();
    let would_remove = results
        .iter()
        .filter(|r| r["status"] == "would remove")
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
            &serde_json::json!({"schema_version":2,"operation":"git.worktree.clean","status":status,"apply":args.apply,"scope":presentation::paths(&args.paths),"protected":presentation::paths(&selected.protected),"results":results,"summary":{"removed":removed,"would_remove":would_remove,"blocked":blocked,"failed":failed,"unverified":unverified,"errors":errors,"interrupted":interrupted,"elapsed_seconds":started.elapsed().as_secs_f64()}}),
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
            "{}{}\n  {} {}\n  {blocked} blocked",
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
    }
    Ok(code)
}
