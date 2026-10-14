//! Auth subprocesses keep credentials in bounded memory, never in capture files.
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

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
pub(crate) fn capture(mut command: Command, timeout: u64, login: bool) -> Result<Output, String> {
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
    let mut child = command
        .spawn()
        .map_err(|_| "Auth CLI could not be launched; check installation and permissions")?;
    let (send, receive) = mpsc::sync_channel(16);
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
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
                if writeln!(std::io::stderr(), "Complete authentication: {url}").is_err() {
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
        match child.try_wait() {
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
    let stopped = crate::runtime::stop_group(&mut child);
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
