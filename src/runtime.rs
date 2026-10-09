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
        if error.raw_os_error() != Some(libc::ESRCH)
            && !inert_group_permission_error(child.id(), &error)?
        {
            return Err(format!("cannot terminate worker group: {error}"));
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    // Reaping the leader does not terminate its descendants. Verify that no
    // live group members remain before releasing repository coordination.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    while !group_is_inert(child.id())? {
        if std::time::Instant::now() >= deadline {
            return Err("worker group still active after termination".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    Ok(status)
}

// Darwin's killpg skips zombies and returns EPERM when no live member remains.
// Never suppress a permission error unless the process table proves that case.
fn inert_group_permission_error(group: u32, error: &io::Error) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    if error.raw_os_error() == Some(libc::EPERM) {
        return group_is_inert(group);
    }
    let _ = (group, error);
    Ok(false)
}

#[cfg(target_os = "macos")]
fn group_is_inert(group: u32) -> Result<bool, String> {
    const PROC_PGRP_ONLY: u32 = 2; // sys/proc_info.h
    let mut capacity = 64;
    loop {
        let mut pids = vec![0 as libc::pid_t; capacity];
        let size = std::mem::size_of_val(pids.as_slice());
        unsafe { *libc::__error() = 0 };
        let bytes = unsafe {
            libc::proc_listpids(PROC_PGRP_ONLY, group, pids.as_mut_ptr().cast(), size as i32)
        };
        if bytes < 0 || (bytes == 0 && io::Error::last_os_error().raw_os_error() != Some(0)) {
            return Err(format!(
                "cannot inspect worker group: {}",
                io::Error::last_os_error()
            ));
        }
        if bytes as usize % std::mem::size_of::<libc::pid_t>() != 0 {
            return Err("cannot inspect worker group: incomplete process table".into());
        }
        if bytes as usize >= size {
            if capacity >= 65_536 {
                return Err("cannot inspect complete worker group: process table changed".into());
            }
            capacity *= 2;
            continue;
        }
        for pid in pids
            .into_iter()
            .take(bytes as usize / std::mem::size_of::<libc::pid_t>())
        {
            let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
            let size = std::mem::size_of_val(&info) as i32;
            let read = unsafe {
                libc::proc_pidinfo(
                    pid,
                    libc::PROC_PIDTBSDINFO,
                    0,
                    (&mut info as *mut libc::proc_bsdinfo).cast(),
                    size,
                )
            };
            if read == size {
                if info.pbi_pgid == group && info.pbi_status != libc::SZOMB {
                    return Ok(false);
                }
            } else if read != 0 || io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
                return Err(format!(
                    "cannot inspect worker group member {pid}: {}",
                    io::Error::last_os_error()
                ));
            }
            // proc_pidinfo returns ESRCH for zombies and members that exited.
        }
        return Ok(true);
    }
}

#[cfg(target_os = "linux")]
fn group_is_inert(group: u32) -> Result<bool, String> {
    if unsafe { libc::kill(-(group as libc::pid_t), 0) } == -1
        && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
    {
        return Ok(true);
    }
    for entry in
        std::fs::read_dir("/proc").map_err(|e| format!("cannot inspect worker group: {e}"))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_name().to_string_lossy().parse::<u32>().is_err() {
            continue;
        }
        let stat = match std::fs::read_to_string(entry.path().join("stat")) {
            Ok(stat) => stat,
            Err(e)
                if e.kind() == io::ErrorKind::NotFound || e.raw_os_error() == Some(libc::ESRCH) =>
            {
                continue
            }
            Err(e) => return Err(format!("cannot inspect worker group member: {e}")),
        };
        // comm may contain spaces or parentheses; the fields follow its last ')'.
        let fields = stat
            .rsplit_once(") ")
            .ok_or("invalid process table record")?
            .1
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err("incomplete process table record".into());
        }
        let pgrp = fields[2]
            .parse::<u32>()
            .map_err(|_| "invalid process group")?;
        if pgrp == group && !matches!(fields[0], "Z" | "X") {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn group_is_inert(_: u32) -> Result<bool, String> {
    Err("worker group inspection is unsupported on this platform".into())
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

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::os::unix::process::CommandExt;

    #[test]
    fn exited_group_with_unreaped_descendant_is_not_a_permission_failure() {
        struct Leader(std::process::Child);
        impl Drop for Leader {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut owned = Leader(
            std::process::Command::new("/bin/sleep")
                .arg("30")
                .process_group(0)
                .spawn()
                .unwrap(),
        );
        let leader = &mut owned.0;
        let group = leader.id() as libc::pid_t;
        assert!(!group_is_inert(group as u32).unwrap());
        assert!(!inert_group_permission_error(
            group as u32,
            &io::Error::from_raw_os_error(libc::EPERM)
        )
        .unwrap());
        let zombie = unsafe { libc::fork() };
        assert!(zombie >= 0);
        if zombie == 0 {
            // Only async-signal-safe operations after fork in the test runner.
            let joined = unsafe { libc::setpgid(0, group) };
            unsafe { libc::_exit(if joined == 0 { 0 } else { 1 }) };
        }
        struct Reap(libc::pid_t);
        impl Drop for Reap {
            fn drop(&mut self) {
                unsafe { libc::waitpid(self.0, std::ptr::null_mut(), 0) };
            }
        }
        let _reap = Reap(zombie);
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        assert_eq!(
            unsafe {
                libc::waitid(
                    libc::P_PID,
                    zombie as _,
                    &mut info,
                    libc::WEXITED | libc::WNOWAIT,
                )
            },
            0
        );
        leader.kill().unwrap();
        leader.wait().unwrap();
        assert_eq!(unsafe { libc::kill(-group, libc::SIGKILL) }, -1);
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::EPERM));
        assert!(stop_group(leader).is_ok());
    }
}
