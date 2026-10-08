//! Process lifetime and cancellation, independent of repository policy.
use std::io;
use std::process::{Child, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static CANCELLED: AtomicBool = AtomicBool::new(false);
extern "C" fn stop(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::SeqCst);
    CANCELLED.store(true, Ordering::SeqCst);
}
pub(crate) fn cancelled() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}
pub(crate) struct Signals {
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

pub(crate) fn exited(child: &mut Child) -> Result<bool, String> {
    // Use the previous home supervisor's ordering on both supported platforms:
    // observe/reap the leader, then stop its group before releasing the lock.
    child
        .try_wait()
        .map(|status| status.is_some())
        .map_err(|e| e.to_string())
}

pub(crate) fn stop_group(child: &mut Child) -> Result<ExitStatus, String> {
    let result = unsafe { libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL) };
    if result != 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(format!("cannot terminate worker group: {error}"));
        }
    }
    child.wait().map_err(|e| e.to_string())
}

pub(crate) fn cancel() {
    CANCELLED.store(true, Ordering::SeqCst);
}
pub(crate) fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// Bound a command's lifetime and clean descendants before returning captures.
/// Worker commands already belong to an enclosing supervised group.
pub(crate) fn capture(
    mut cmd: std::process::Command,
    timeout: std::time::Duration,
    own_group: bool,
) -> Result<(i32, Vec<u8>, Vec<u8>), String> {
    use std::io::{Read, Seek};
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    let mut out = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut err = tempfile::tempfile().map_err(|e| e.to_string())?;
    cmd.stdout(out.try_clone().map_err(|e| e.to_string())?)
        .stderr(err.try_clone().map_err(|e| e.to_string())?)
        .stdin(Stdio::null());
    if own_group {
        cmd.process_group(0);
    }
    if cancelled() {
        return Err("Interrupted before subprocess launch".into());
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot execute subprocess: {e}"))?;
    let deadline = std::time::Instant::now() + timeout;
    let code = if !own_group {
        child.wait().map_err(|e| e.to_string())?.code().unwrap_or(1)
    } else {
        loop {
            match exited(&mut child) {
                Ok(true) => break stop_group(&mut child)?.code().unwrap_or(1),
                Ok(false) => {}
                Err(e) => {
                    let _ = stop_group(&mut child);
                    return Err(e);
                }
            }
            if cancelled() || std::time::Instant::now() >= deadline {
                stop_group(&mut child)?;
                return Err(if cancelled() {
                    "Interrupted; process group terminated"
                } else {
                    "Deadline exceeded; process group terminated"
                }
                .into());
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    };
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    out.rewind()
        .and_then(|_| out.read_to_end(&mut stdout))
        .map_err(|e| e.to_string())?;
    err.rewind()
        .and_then(|_| err.read_to_end(&mut stderr))
        .map_err(|e| e.to_string())?;
    Ok((code, stdout, stderr))
}
