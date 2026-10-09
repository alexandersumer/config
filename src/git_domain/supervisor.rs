//! Bounded worker processes, shared repository locks, and group lifetime ownership.
use super::{
    discover::{Repo, Selection},
    display, output,
    status::Status,
    Options,
};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::os::unix::{fs::OpenOptionsExt, process::CommandExt};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Arc, Mutex, OnceLock,
};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::runtime::{cancel, cancelled, exited, interrupted, stop_group};
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
            cancel();
        }
    }
}

struct Diagnostics {
    directory: Option<tempfile::TempDir>,
    reported: bool,
}
impl Drop for Diagnostics {
    fn drop(&mut self) {
        if let Some(directory) = self.directory.take() {
            if fs::read_dir(directory.path()).map_or(true, |mut entries| entries.next().is_some()) {
                let path = directory.keep();
                if !self.reported {
                    let _ = writeln!(io::stderr(), "Logs: {}", display(path));
                }
            }
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
    let lock = super::lock::acquire(&repo.common)?;
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
        let mut command = Command::new(executable);
        command
            .args([
                "--internal-reset-worker",
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
            let code = if interrupted() { 130 } else { 1 };
            if options.json {
                let _ = crate::presentation::json(
                    &serde_json::json!({"schema_version":1,"operation":"git.reset","status":if code==130 {"interrupted"}else{"failed"},"error":error}),
                );
            }
            code
        }
    }
}
fn run_inner(options: &Options, selection: Selection) -> Result<u8, String> {
    let mut overview: Box<dyn Write> = if options.json {
        Box::new(io::stderr())
    } else {
        Box::new(io::stdout())
    };
    writeln!(overview,"Git checkout reset\n\nScope          {}\nCheckouts      {}\nExcluded       {} linked worktrees\nWorkers        {}\n",
        options.paths.iter().map(|p|output::short(p)).collect::<Vec<_>>().join(" "),selection.repos.len(),selection.excluded.len(),options.jobs.min(selection.repos.len())).map_err(|e|e.to_string())?;
    overview.flush().map_err(|e| e.to_string())?;
    drop(overview);
    let total = selection.repos.len();
    let started = Instant::now();
    if total == 0 {
        if options.json {
            return Ok(crate::presentation::json(
                &serde_json::json!({"schema_version":1,"operation":"git.reset","status":"completed","scope":crate::presentation::paths(&options.paths),"results":[],"excluded":crate::presentation::paths(&selection.excluded),"summary":{"succeeded":0,"failed":0,"cancelled":0,"elapsed_seconds":0.0}}),
            ));
        }
        writeln!(
            io::stdout(),
            "Completed\n  No repositories to reset.\n  0 succeeded\n  0 failed\n  Elapsed 0s"
        )
        .map_err(|e| e.to_string())?;
        return Ok(0);
    }
    let directory = tempfile::Builder::new()
        .prefix("workctl-reset-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let logs = directory.path().to_owned();
    let mut diagnostics = Diagnostics {
        directory: Some(directory),
        reported: false,
    };
    if options.keep_logs {
        writeln!(io::stderr(), "Logs: {}", display(&logs)).map_err(|e| e.to_string())?;
        diagnostics.reported = true;
    }
    writeln!(io::stderr(), "Resetting checkouts…").map_err(|e| e.to_string())?;
    let resources: HashMap<PathBuf, Arc<Mutex<()>>> = selection
        .repos
        .iter()
        .map(|repo| (repo.common.clone(), Arc::new(Mutex::new(()))))
        .collect();
    let next = AtomicUsize::new(0);
    let (tx, rx) = mpsc::channel();
    let mut rendering = crate::presentation::Progress::new(total);
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
                    cancel();
                    break;
                }
                let mut progress = Progress::default();
                let outcome = if cancelled() {
                    Ok((130, 0, "Cancelled before execution".into()))
                } else {
                    execute(options, repo, &log, &mut progress)
                };
                let (code, attempts, detail) =
                    outcome.unwrap_or_else(|e| (1, progress.attempts, e));
                if !log.exists() {
                    let _ = fs::write(&log, format!("{detail}\n"));
                }
                let record = ResultRecord {
                    path: repo.path.clone(),
                    code,
                    attempts,
                    seconds: started.elapsed().as_secs_f64(),
                    log,
                    detail,
                    target: progress.target,
                    backups: progress.backups,
                };
                if tx.send(Event::Finished(index, record)).is_err() {
                    cancel();
                    break;
                }
            });
        }
        drop(tx);
        let mut records = Vec::with_capacity(total);
        let mut active = HashSet::new();
        while records.len() < total {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(Event::Started(index)) => {
                    active.insert(index);
                }
                Ok(Event::Finished(index, record)) => {
                    active.remove(&index);
                    if record.code != 0 {
                        rendering.message(&format!(
                            "\n  {} {}\n{}\n    Log: {}",
                            crate::presentation::status(
                                if record.code == 130 {
                                    "Cancelled:"
                                } else {
                                    "Failed:"
                                },
                                true,
                                true
                            ),
                            display(&record.path),
                            output::wrap(&display(&record.detail)),
                            display(&record.log)
                        ))?;
                    }
                    records.push((index, record));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(e) => return Err(format!("Worker reporting failed: {e}")),
            }
            rendering.update(records.len(), active.len())?;
        }
        records.sort_by_key(|(i, _)| *i);
        cancel_guard.0 = false;
        Ok(records.into_iter().map(|(_, r)| r).collect())
    })?;
    rendering.finish();
    if options.verbose {
        for record in &records {
            let text = fs::read(&record.log).unwrap_or_default();
            writeln!(
                io::stderr(),
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
    let ok = records.iter().filter(|r| r.code == 0).count();
    let stopped = records.iter().filter(|r| r.code == 130).count();
    let code = if cancelled() || stopped > 0 {
        130
    } else {
        u8::from(ok != total)
    };
    let retain = code != 0 || options.keep_logs;
    // Successful-run results never refer to diagnostics that have been deleted.
    let result:Vec<_>=records.iter().map(|r|serde_json::json!({"path":r.path.to_string_lossy(),"path_bytes":crate::presentation::path_bytes(&r.path),"code":r.code,"status":if r.code==0 {"succeeded"}else if r.code==130 {"cancelled"}else{"failed"},"attempts":r.attempts,"seconds":r.seconds,"log":if retain {Some(r.log.to_string_lossy())}else{None},"detail":r.detail,"target":r.target,"backups":r.backups})).collect();
    fs::write(
        logs.join("results.json"),
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        logs.join("excluded-worktrees.json"),
        serde_json::to_vec_pretty(&crate::presentation::paths(&selection.excluded))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if !retain {
        fs::remove_dir_all(&logs)
            .map_err(|e| format!("could not remove temporary logs at {}: {e}", display(&logs)))?;
        diagnostics.directory.take();
    } else {
        if !options.keep_logs {
            writeln!(io::stderr(), "Logs: {}", display(&logs)).map_err(|e| e.to_string())?;
        }
        drop(diagnostics.directory.take().unwrap().keep());
    }
    let outcome = if code == 130 {
        "Interrupted"
    } else if code != 0 {
        "Completed with failures"
    } else {
        "Completed"
    };
    if options.json {
        if crate::presentation::json(
            &serde_json::json!({"schema_version":1,"operation":"git.reset","status":if code==0 {"completed"}else if code==130 {"interrupted"}else{"failed"},"scope":crate::presentation::paths(&options.paths),"excluded":crate::presentation::paths(&selection.excluded),"results":result,"diagnostics":if retain {Some(logs.to_string_lossy())}else{None},"summary":{"succeeded":ok,"failed":total-ok-stopped,"cancelled":stopped,"elapsed_seconds":started.elapsed().as_secs_f64()}}),
        ) != 0
        {
            return Ok(1);
        }
    } else {
        let outcome = crate::presentation::status(outcome, code != 0, false);
        let mut out = io::stdout().lock();
        writeln!(
            out,
            "{outcome}\n  {ok} succeeded\n  {} failed",
            total - ok - stopped
        )
        .map_err(|e| e.to_string())?;
        if stopped > 0 {
            writeln!(out, "  {stopped} cancelled").map_err(|e| e.to_string())?;
        }
        writeln!(
            out,
            "  Elapsed {}",
            crate::presentation::duration(started.elapsed())
        )
        .map_err(|e| e.to_string())?;
        let backups: usize = records.iter().map(|record| record.backups.len()).sum();
        if backups > 0 {
            writeln!(
                out,
                "  {backups} recovery refs saved{}",
                if options.verbose {
                    ""
                } else {
                    " (--verbose shows recovery commands)"
                }
            )
            .map_err(|e| e.to_string())?;
        }
        for record in &records {
            if total == 1 && record.code == 0 {
                writeln!(
                    out,
                    "  Reset {} to {}",
                    display(&record.path),
                    display(record.target.as_deref().unwrap_or("remote target"))
                )
                .map_err(|e| e.to_string())?;
            }
            for backup in record
                .backups
                .iter()
                .filter(|_| options.verbose || record.code != 0)
            {
                writeln!(
                    out,
                    "  Recover {}: git branch recovered-work {}",
                    display(&record.path),
                    display(backup)
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(code)
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
