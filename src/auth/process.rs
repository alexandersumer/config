//! Auth subprocesses keep credentials in bounded memory, never in capture files.
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

/// Shared by one preflight; provider workers never share credential operations.
#[derive(Default)]
pub(crate) struct Session {
    interaction: Mutex<()>,
    cleanup_failed: AtomicBool,
}
impl Session {
    pub fn message(&self, message: &str) -> Result<(), String> {
        // A single locked write keeps concurrent progress and prompts intact.
        writeln!(std::io::stderr().lock(), "{message}")
            .map_err(|_| "Cannot display authentication progress".into())
    }
    fn interactive(&self) -> Result<MutexGuard<'_, ()>, String> {
        loop {
            if crate::runtime::cancelled() {
                return Err("Interrupted waiting for authentication interaction".into());
            }
            match self.interaction.try_lock() {
                Ok(guard) => return Ok(guard),
                Err(TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(10)),
                Err(TryLockError::Poisoned(_)) => {
                    self.fail_cleanup();
                    return Err("Authentication interaction worker failed".into());
                }
            }
        }
    }
    pub fn fail_cleanup(&self) {
        self.cleanup_failed.store(true, Ordering::SeqCst);
        crate::runtime::cancel();
    }
    pub fn cleanup_failed(&self) -> bool {
        self.cleanup_failed.load(Ordering::SeqCst)
    }
}

const LIMIT: usize = 256 * 1024;
struct Capture {
    bytes: Vec<u8>,
    overflow: bool,
}
fn drain(
    mut pipe: impl Read + AsRawFd,
    prompts: mpsc::SyncSender<String>,
    stop: Arc<AtomicBool>,
) -> Result<Capture, ()> {
    let fd = pipe.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(());
    }
    let mut bytes = Vec::new();
    let mut line = Vec::new();
    let mut buffer = [0; 4096];
    let mut overflow = false;
    loop {
        let n = match pipe.read(&mut buffer) {
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
                continue;
            }
            Err(_) => return Err(()),
        };
        if stop.load(Ordering::SeqCst) && overflow {
            break;
        }
        if n == 0 {
            break;
        }
        let available = LIMIT.saturating_sub(bytes.len());
        bytes.extend_from_slice(&buffer[..n.min(available)]);
        overflow |= n > available;
        for &b in &buffer[..n] {
            if b == b'\n' {
                // The activation code is intended for the user. No arbitrary
                // provider output, tokens, or OAuth callback URLs are forwarded.
                if let Some(url) = activation_url(&line) {
                    let _ = prompts.try_send(url);
                }
                line.clear();
            } else if line.len() < 4096 {
                line.push(b);
            }
        }
    }
    if let Some(url) = activation_url(&line) {
        let _ = prompts.try_send(url);
    }
    Ok(Capture { bytes, overflow })
}
fn activation_url(line: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(line).ok()?;
    let prefix = "https://atlassian.okta.com/activate?user_code=";
    let code = text.split_once(prefix)?.1.trim();
    if (4..=32).contains(&code.len())
        && code
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
    {
        Some(format!("{prefix}{code}"))
    } else {
        None
    }
}
pub(crate) struct Output {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
// Keep process ownership during unwinding too; releasing the interaction gate
// must never leave a live provider behind.
struct ProviderChild<'a> {
    child: std::process::Child,
    session: &'a Session,
    cleaned: bool,
}
impl ProviderChild<'_> {
    fn stop(&mut self) -> Result<std::process::ExitStatus, String> {
        let result = crate::runtime::stop_group(&mut self.child);
        if result.is_ok() {
            self.cleaned = true;
        } else {
            self.session.fail_cleanup();
        }
        result
    }
}
impl Drop for ProviderChild<'_> {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.stop();
        }
    }
}
pub(crate) fn capture(
    session: &Session,
    label: &str,
    mut command: Command,
    timeout: u64,
    login: bool,
) -> Result<Output, String> {
    // Acquire before launching any flow, and hold through descendant cleanup.
    // No command deadline runs while waiting for another interactive operation.
    let _interaction = if login {
        Some(session.interactive()?)
    } else {
        None
    };
    if login {
        session.message(&format!(
            "    {}: Complete browser sign-in or MFA if prompted ({timeout}s timeout).",
            super::inline(label)
        ))?;
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("TERM", "dumb")
        .env("NO_COLOR", "1")
        .process_group(0);
    if crate::runtime::cancelled() {
        return Err("Interrupted before auth command".into());
    }
    // Spawn errors can include user-controlled filenames; do not echo them.
    let mut child = ProviderChild {
        child: command
            .spawn()
            .map_err(|_| "Auth CLI could not be launched; check installation and permissions")?,
        session,
        cleaned: false,
    };
    let (send, receive) = mpsc::sync_channel(16);
    let stdout = child.child.stdout.take().expect("piped stdout");
    let stderr = child.child.stderr.take().expect("piped stderr");
    let other = send.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_out = stop.clone();
    let stop_err = stop.clone();
    let out = std::thread::spawn(move || drain(stdout, send, stop_out));
    let err = std::thread::spawn(move || drain(stderr, other, stop_err));
    let deadline = Instant::now() + Duration::from_secs(timeout);
    let mut failure = None;
    let mut prompt_seen = false;
    loop {
        for url in receive.try_iter() {
            if login && !prompt_seen {
                prompt_seen = true;
                if session
                    .message(&format!(
                        "Complete authentication: {url} ({})",
                        super::inline(label)
                    ))
                    .is_err()
                {
                    failure = Some("Cannot display authentication prompt");
                }
            }
        }
        if crate::runtime::cancelled() {
            failure = Some("Interrupted during auth command");
        }
        if Instant::now() >= deadline {
            failure = Some("Auth command deadline exceeded");
        }
        match child.child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if failure.is_none() => std::thread::sleep(Duration::from_millis(10)),
            Ok(None) => break,
            Err(_) => {
                failure = Some("Cannot supervise auth command");
                break;
            }
        }
    }
    // Terminate descendants even when the provider leader exits first.
    let stopped = child.stop();
    stop.store(true, Ordering::SeqCst);
    // Nonblocking readers also stop if a faulty provider detaches with an open pipe.
    let stdout = out
        .join()
        .map_err(|_| "Auth output reader failed")?
        .map_err(|_| "Cannot read auth output")?;
    let stderr = err
        .join()
        .map_err(|_| "Auth error reader failed")?
        .map_err(|_| "Cannot read auth diagnostics")?;
    let status = stopped.map_err(|_| "Cannot confirm auth process cleanup")?;
    if let Some(reason) = failure {
        return Err(reason.into());
    }
    if stdout.overflow || stderr.overflow {
        return Err("Auth CLI output exceeds safety limit".into());
    }
    Ok(Output {
        code: status.code().unwrap_or(1),
        stdout: stdout.bytes,
        stderr: stderr.bytes,
    })
}

pub(crate) struct Lock(std::fs::File);
impl Lock {
    pub fn acquire(timeout: u64) -> Result<Self, String> {
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        let uid = unsafe { libc::geteuid() };
        let path = std::env::temp_dir().join(format!("workctl-auth-{uid}.lock"));
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
            .map_err(|_| "Cannot open auth coordination lock")?;
        let metadata = file
            .metadata()
            .map_err(|_| "Cannot inspect auth coordination lock")?;
        if !metadata.is_file()
            || metadata.uid() != uid
            || metadata.nlink() != 1
            || metadata.mode() & 0o777 != 0o600
        {
            return Err(
                "Unsafe auth coordination lock; expected an owned private regular file".into(),
            );
        }
        let deadline = Instant::now() + Duration::from_secs(timeout);
        loop {
            if crate::runtime::cancelled() {
                return Err("Interrupted waiting for auth coordination".into());
            }
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                return Ok(Self(file));
            }
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::EWOULDBLOCK) {
                return Err("Cannot acquire auth coordination lock".into());
            }
            if Instant::now() >= deadline {
                return Err("Auth refresh already running; coordination deadline exceeded".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interaction_wait_does_not_consume_the_command_deadline() {
        let session = Session::default();
        let gate = session.interaction.lock().unwrap();
        let (entered, waiting) = mpsc::sync_channel(1);
        let marker = tempfile::NamedTempFile::new().unwrap();
        let marker_path = marker.path().to_owned();
        std::thread::scope(|scope| {
            let session = &session;
            let worker = scope.spawn(|| {
                let mut command = Command::new("/bin/sh");
                command
                    .args(["-c", "printf started > \"$1\"; printf ready", "sh"])
                    .arg(&marker_path);
                entered.send(()).unwrap();
                capture(session, "test", command, 1, true)
            });
            waiting.recv().unwrap();
            // Longer than the command budget, but no provider may launch yet.
            std::thread::sleep(Duration::from_millis(1200));
            assert_eq!(std::fs::read(&marker_path).unwrap(), b"");
            drop(gate);
            let output = worker.join().unwrap().unwrap();
            assert_eq!(output.code, 0);
            assert_eq!(output.stdout, b"ready");
        });
    }
}
