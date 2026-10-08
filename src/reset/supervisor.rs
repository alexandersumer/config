//! Bounded worker processes, shared repository locks, and group lifetime ownership.
use super::{
    discover::{Repo, Selection},
    display, output,
    status::Status,
    Options,
};
use regex::Regex;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::os::unix::{fs::OpenOptionsExt, process::CommandExt};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc, Arc, Mutex, OnceLock,
};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static CANCELLED: AtomicBool = AtomicBool::new(false);
extern "C" fn stop(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::SeqCst);
    CANCELLED.store(true, Ordering::SeqCst);
}
pub(super) fn cancelled() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}
pub(super) struct Signals {
    old: Vec<(i32, libc::sighandler_t)>,
}
impl Signals {
    pub fn install() -> Result<Self, String> {
        CANCELLED.store(false, Ordering::SeqCst);
        INTERRUPTED.store(false, Ordering::SeqCst);
        let mut signals = Self { old: vec![] };
        for signal in [libc::SIGINT, libc::SIGTERM] {
            // Handler only performs an atomic store, no allocation or locking.
            let old = unsafe { libc::signal(signal, stop as libc::sighandler_t) };
            if old == libc::SIG_ERR {
                return Err(io::Error::last_os_error().to_string());
            }
            signals.old.push((signal, old));
        }
        Ok(signals)
    }
}
impl Drop for Signals {
    fn drop(&mut self) {
        for &(signal, old) in &self.old {
            unsafe {
                libc::signal(signal, old);
            }
        }
    }
}

pub(super) fn exited(child: &mut Child) -> Result<bool, String> {
    // Use the previous home supervisor's ordering on both supported platforms:
    // observe/reap the leader, then stop its group before releasing the lock.
    child
        .try_wait()
        .map(|status| status.is_some())
        .map_err(|e| e.to_string())
}

pub(super) fn stop_group(child: &mut Child) -> Result<ExitStatus, String> {
    let result = unsafe { libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL) };
    if result != 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(format!("cannot terminate worker group: {error}"));
        }
    }
    child.wait().map_err(|e| e.to_string())
}
struct OwnedChild(Child, bool);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !self.1 {
            let _ = stop_group(&mut self.0);
        }
    }
}
struct CancelOnDrop(bool);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if self.0 {
            CANCELLED.store(true, Ordering::SeqCst);
        }
    }
}

enum Event {
    Started(usize),
    Finished(usize, ResultRecord),
}

struct ResultRecord {
    path: PathBuf,
    code: i32,
    attempts: usize,
    seconds: f64,
    log: PathBuf,
    detail: String,
    target: Option<String>,
    backups: Vec<String>,
}

fn retryable(text: &str) -> bool {
    static PERMANENT: OnceLock<Regex> = OnceLock::new();
    static TRANSIENT: OnceLock<Regex> = OnceLock::new();
    let permanent = PERMANENT.get_or_init(|| Regex::new(r"(?i)Permission denied|Access denied|Forbidden|Authentication failed|could not read Username|Host key verification failed|You may not have access to this repository|refusing|would be overwritten|Git operation in progress|cannot lock|Repository is busy").unwrap());
    let transient = TRANSIENT.get_or_init(|| Regex::new(r"(?i)Could not resolve|Connection (?:timed out|reset|refused|closed)|Operation timed out|early EOF|unexpected disconnect|remote end hung up|TLS connection was non-properly terminated|(?:HTTP/[\d.]+\s+|returned error:?\s*)(?:5\d\d|429)\b|Failed to connect|Couldn't connect|kex_exchange_identification").unwrap());
    !permanent.is_match(text) && transient.is_match(text)
}

#[derive(Default)]
struct Progress {
    attempts: usize,
    target: Option<String>,
    backups: Vec<String>,
}
fn execute(
    options: &Options,
    repo: &Repo,
    log: &PathBuf,
    progress: &mut Progress,
) -> Result<(i32, usize, String), String> {
    let mut output = OpenOptions::new()
        .create_new(true)
        .append(true)
        .read(true)
        .mode(0o600)
        .open(log)
        .map_err(|e| e.to_string())?;
    let lock = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(repo.common.join("repo-batch.lock"))
        .map_err(|e| e.to_string())?;
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        let error = io::Error::last_os_error();
        return Err(format!("Repository is busy or cannot be locked: {error}"));
    }
    let status_path = log.with_extension("status.json");
    let mut attempts = 0;
    let mut code = 130;
    let mut detail = "Cancelled before execution".to_string();
    while attempts < options.attempts && !cancelled() {
        attempts += 1;
        writeln!(output, "Attempt {attempts}/{}", options.attempts).map_err(|e| e.to_string())?;
        Status {
            stage: "worker startup".into(),
            ..Status::default()
        }
        .write(&status_path)?;
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let standalone = executable
            .file_name()
            .is_some_and(|n| n == "reset_to_origin");
        let mut command = Command::new(executable);
        if !standalone {
            command.arg("reset-to-origin");
        }
        command
            .args([
                "--internal-worker",
                &options.remote,
                options.branch.as_deref().unwrap_or(""),
            ])
            .current_dir(&repo.path)
            .stdin(Stdio::null())
            .stdout(output.try_clone().map_err(|e| e.to_string())?)
            .stderr(output.try_clone().map_err(|e| e.to_string())?)
            .process_group(0);
        let fd = lock.as_raw_fd();
        command.env("RESET_TO_ORIGIN_WORKER_LOCK", fd.to_string());
        command.env("RESET_TO_ORIGIN_WORKER_STATUS", &status_path);
        unsafe {
            command.pre_exec(move || {
                if libc::fcntl(fd, libc::F_SETFD, 0) == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        if cancelled() {
            return Ok((130, attempts - 1, "Cancelled before execution".into()));
        }
        progress.attempts = attempts;
        let mut child = OwnedChild(
            command
                .spawn()
                .map_err(|e| format!("cannot launch worker: {e}"))?,
            false,
        );
        let deadline = Instant::now() + options.timeout;
        code = loop {
            if cancelled() {
                break 130;
            }
            if Instant::now() >= deadline {
                break 124;
            }
            if exited(&mut child.0)? {
                let status = stop_group(&mut child.0)?;
                child.1 = true;
                break status.code().unwrap_or(1);
            }
            thread::sleep(Duration::from_millis(10));
        };
        if !child.1 {
            stop_group(&mut child.0)?;
            child.1 = true;
        }
        let status = Status::read(&status_path)?;
        if let Some(target) = status.target {
            progress.target = Some(target);
        }
        for backup in status.backups {
            if !progress.backups.contains(&backup) {
                progress.backups.push(backup);
            }
        }
        let last_stage = &status.stage;
        let last_state = status.state.as_deref();
        // A successful process exit alone is insufficient: it must publish a
        // terminal record. Timeouts and cancellation are owned by this process.
        let terminal_matches = status.complete && ((code == 0) == status.error.is_none());
        if code == 0 && !terminal_matches {
            code = 1;
        }
        detail = match code {
            124 => format!(
                "Timed out during {last_stage}; process group terminated{}",
                last_state.map(|s| format!("; {s}")).unwrap_or_default()
            ),
            130 => format!(
                "Cancelled during {last_stage}; process group terminated{}",
                last_state.map(|s| format!("; {s}")).unwrap_or_default()
            ),
            _ if !terminal_matches => {
                "Worker exited without a matching terminal status; inspect its log".into()
            }
            0 => "Reset completed".into(),
            _ => status.error.clone().unwrap(),
        };
        writeln!(output, "Outcome: {detail}").map_err(|e| e.to_string())?;
        if code == 0
            || code == 130
            || attempts == options.attempts
            || (code != 124 && !status.error.as_deref().is_some_and(retryable))
        {
            break;
        }
        let base = (2u64 << (attempts - 1)).min(30) as f64;
        let jitter = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as f64
            / 1_000_000_000.0;
        let delay = Duration::from_secs_f64(base + jitter * (base / 4.0).min(1.0));
        writeln!(
            output,
            "Temporary failure; retrying in {:.1}s",
            delay.as_secs_f64()
        )
        .map_err(|e| e.to_string())?;
        let until = Instant::now() + delay;
        while Instant::now() < until && !cancelled() {
            thread::sleep(Duration::from_millis(25));
        }
        if cancelled() {
            code = 130;
            detail = "Cancelled during retry backoff".into();
        }
    }
    Ok((code, attempts, detail))
}

pub(super) fn run(options: &Options, selection: Selection) -> u8 {
    match run_inner(options, selection) {
        Ok(code) => code,
        Err(error) => {
            super::report(&error);
            if INTERRUPTED.load(Ordering::SeqCst) {
                130
            } else {
                1
            }
        }
    }
}
fn run_inner(options: &Options, selection: Selection) -> Result<u8, String> {
    let mut stdout = io::stdout().lock();
    if !selection.excluded.is_empty() {
        writeln!(
            stdout,
            "{}",
            output::color(
                &format!(
                    "Skipped {}.",
                    output::quantity(selection.excluded.len(), "linked worktree")
                ),
                33
            )
        )
        .map_err(|e| e.to_string())?;
        if options.verbose {
            for path in &selection.excluded {
                writeln!(stdout, "  Skipped: {}", output::short(path))
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    if selection.repos.is_empty() {
        writeln!(stdout, "No repositories to reset.").map_err(|e| e.to_string())?;
        return Ok(0);
    }
    let logs = tempfile::Builder::new()
        .prefix("reset-to-origin-")
        .tempdir()
        .map_err(|e| e.to_string())?
        .keep();
    let started = Instant::now();
    writeln!(
        stdout,
        "Resetting {} with {}.",
        output::quantity(selection.repos.len(), "repository"),
        output::quantity(options.jobs.min(selection.repos.len()), "worker")
    )
    .map_err(|e| e.to_string())?;
    for scope in &selection.scopes {
        writeln!(
            stdout,
            "  {} ({})",
            output::short(&scope.path),
            output::quantity(scope.count, "repository")
        )
        .map_err(|e| e.to_string())?;
    }
    if options.keep_logs {
        writeln!(stdout, "Logs: {}", display(&logs)).map_err(|e| e.to_string())?;
    }
    let resources: HashMap<PathBuf, Arc<Mutex<()>>> = selection
        .repos
        .iter()
        .map(|repo| (repo.common.clone(), Arc::new(Mutex::new(()))))
        .collect();
    let next = AtomicUsize::new(0);
    let (tx, rx) = mpsc::channel();
    let total = selection.repos.len();
    let records = thread::scope(|scope| -> Result<Vec<ResultRecord>, String> {
        let mut cancel_guard = CancelOnDrop(true);
        for _ in 0..options.jobs.min(total) {
            let tx = tx.clone();
            let repos = &selection.repos;
            let next = &next;
            let logs = &logs;
            let resources = &resources;
            scope.spawn(move || loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(repo) = repos.get(index) else {
                    break;
                };
                let log = logs.join(format!("{:04}.log", index + 1));
                let started = Instant::now();
                let _guard = resources[&repo.common].lock().unwrap();
                if tx.send(Event::Started(index)).is_err() {
                    CANCELLED.store(true, Ordering::SeqCst);
                    break;
                }
                let mut progress = Progress::default();
                let outcome = if cancelled() {
                    Ok((130, 0, "Cancelled before execution".into()))
                } else {
                    execute(options, repo, &log, &mut progress)
                };
                let (code, attempts, detail) = outcome.unwrap_or_else(|error| {
                    if let Ok(mut log) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .mode(0o600)
                        .open(&log)
                    {
                        let _ = writeln!(log, "Worker failure: {error}");
                    }
                    (1, progress.attempts, error)
                });
                if !log.exists() {
                    let _ = fs::write(&log, format!("{detail}\n"));
                }
                let record = ResultRecord {
                    target: progress.target,
                    backups: progress.backups,
                    path: repo.path.clone(),
                    code,
                    attempts,
                    seconds: started.elapsed().as_secs_f64(),
                    log,
                    detail,
                };
                if tx.send(Event::Finished(index, record)).is_err() {
                    CANCELLED.store(true, Ordering::SeqCst);
                    break;
                }
            });
        }
        drop(tx);
        let mut records = Vec::with_capacity(total);
        let mut heartbeat = Instant::now();
        let mut active = BTreeMap::new();
        let mut reported_waits = HashSet::new();
        let mut last_count = 0;
        while records.len() < total {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(Event::Started(index)) => {
                    active.insert(index, Instant::now());
                }
                Ok(Event::Finished(index, record)) => {
                    active.remove(&index);
                    if record.code != 0 {
                        let heading = if record.code == 130 {
                            output::color("Cancelled:", 33)
                        } else {
                            output::color("Failed:", 31)
                        };
                        writeln!(
                            stdout,
                            "\n  {heading} {}\n{}\n    Log: {}\n",
                            output::label(&selection.repos[index], &selection.scopes),
                            output::wrap(&display(&record.detail)),
                            display(&record.log)
                        )
                        .map_err(|e| e.to_string())?;
                    } else if total == 1 {
                        writeln!(
                            stdout,
                            "  {} {} to {}",
                            output::color("Reset", 32),
                            output::label(&selection.repos[index], &selection.scopes),
                            display(record.target.as_deref().unwrap_or("remote target"))
                        )
                        .map_err(|e| e.to_string())?;
                        for backup in &record.backups {
                            writeln!(
                                stdout,
                                "  Previous tip: {}\n  Recover: git branch recovered-work {}",
                                display(backup),
                                display(backup)
                            )
                            .map_err(|e| e.to_string())?;
                        }
                    }
                    records.push((index, record));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(error) => return Err(format!("Worker reporting failed: {error}")),
            }
            if heartbeat.elapsed() >= Duration::from_secs(15) {
                if records.len() != last_count {
                    writeln!(
                        stdout,
                        "  {}/{} complete, elapsed {}s",
                        records.len(),
                        total,
                        started.elapsed().as_secs()
                    )
                    .map_err(|e| e.to_string())?;
                    last_count = records.len();
                } else if let Some((&index, since)) = active
                    .iter()
                    .find(|(index, _)| !reported_waits.contains(*index))
                {
                    let status =
                        Status::read(&logs.join(format!("{:04}.status.json", index + 1))).ok();
                    let stage = status
                        .as_ref()
                        .map(|s| s.stage.as_str())
                        .unwrap_or("worker startup");
                    writeln!(
                        stdout,
                        "  Waiting for {} ({}, {}s).",
                        output::label(&selection.repos[index], &selection.scopes),
                        display(stage),
                        since.elapsed().as_secs()
                    )
                    .map_err(|e| e.to_string())?;
                    reported_waits.insert(index);
                }
                heartbeat = Instant::now();
            }
        }
        records.sort_by_key(|(index, _)| *index);
        // This guard cancels on error; on success workers have already reported.
        cancel_guard.0 = false;
        Ok(records.into_iter().map(|(_, record)| record).collect())
    })?;
    if options.verbose {
        for record in &records {
            let text = fs::read(&record.log).unwrap_or_default();
            writeln!(
                stdout,
                "--- {} ---\n{}",
                display(&record.path),
                String::from_utf8_lossy(&text)
                    .lines()
                    .map(display)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
            .map_err(|e| e.to_string())?;
        }
    }
    let result: Vec<_> = records.iter().map(|r| serde_json::json!({"path":r.path.to_string_lossy(), "code":r.code, "attempts":r.attempts,"seconds":r.seconds,"log":r.log.to_string_lossy(),"detail":r.detail, "target":r.target, "backups":r.backups})).collect();
    fs::write(
        logs.join("results.json"),
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        logs.join("excluded-worktrees.json"),
        serde_json::to_vec_pretty(
            &selection
                .excluded
                .iter()
                .map(|p| p.to_string_lossy())
                .collect::<Vec<_>>(),
        )
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let ok = records.iter().filter(|r| r.code == 0).count();
    let interrupted = records.iter().filter(|r| r.code == 130).count();
    let exit_code = if cancelled() || interrupted > 0 {
        130
    } else {
        u8::from(ok != total)
    };
    if exit_code == 0 && !options.keep_logs {
        fs::remove_dir_all(&logs)
            .map_err(|e| format!("could not remove temporary logs at {}: {e}", display(&logs)))?;
    } else if !options.keep_logs {
        writeln!(stdout, "Logs: {}", display(&logs)).map_err(|e| e.to_string())?;
    }
    let outcome = if interrupted > 0 {
        "Interrupted"
    } else if ok != total {
        "Completed with failures"
    } else {
        "Completed"
    };
    let mut counts = vec![format!("{ok} succeeded")];
    if total - ok - interrupted > 0 {
        counts.push(format!("{} failed", total - ok - interrupted));
    }
    if interrupted > 0 {
        counts.push(format!("{interrupted} cancelled"));
    }
    let summary = format!(
        "{outcome} in {:.2}s: {}.",
        started.elapsed().as_secs_f64(),
        counts.join(", ")
    );
    writeln!(
        stdout,
        "{}",
        output::color(
            &summary,
            if interrupted > 0 {
                33
            } else if ok != total {
                31
            } else {
                32
            }
        )
    )
    .map_err(|e| e.to_string())?;
    let recovered = records
        .iter()
        .filter(|r| r.code == 0 && r.attempts > 1)
        .count();
    if recovered > 0 {
        writeln!(
            stdout,
            "{} recovered after retry.",
            output::quantity(recovered, "repository")
        )
        .map_err(|e| e.to_string())?;
    }
    let backed_up = records.iter().filter(|r| !r.backups.is_empty()).count();
    if total > 1 && backed_up > 0 {
        writeln!(
            stdout,
            "Previous tips saved for {} under refs/home-reset-backups/.",
            output::quantity(backed_up, "repository")
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retry_classifier_never_masks_permanent_errors() {
        for text in [
            "HTTP/2 429",
            "returned error: 503",
            "early EOF",
            "Could not resolve hostname",
        ] {
            assert!(retryable(text));
        }
        for text in [
            "HTTP/2 401",
            "Authentication failed: Connection reset",
            "cannot lock ref",
            "refusing reset; Connection reset",
        ] {
            assert!(!retryable(text));
        }
    }
}
