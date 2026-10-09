//! Common-directory coordination belongs to the Git domain.
use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
pub(super) fn acquire(common: &Path) -> Result<File, String> {
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(common.join("repo-batch.lock"))
        .map_err(|e| {
            format!(
                "cannot open repository coordination lock {}: {e}",
                super::display(common.join("repo-batch.lock"))
            )
        })?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err(format!(
            "Repository coordination lock is not a regular file: {}",
            super::display(common.join("repo-batch.lock"))
        ));
    }
    // Killed descendants can briefly retain inherited file descriptions while
    // the kernel finishes exit. Retry only nonblocking acquisition, briefly.
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(100);
    loop {
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            break;
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EWOULDBLOCK) || std::time::Instant::now() >= deadline
        {
            return Err(format!(
                "Repository is busy or cannot be locked {}: {error}",
                super::display(common.join("repo-batch.lock"))
            ));
        }
        if crate::runtime::cancelled() {
            return Err("Interrupted while acquiring repository coordination lock".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::stop_group;
    use std::os::fd::AsRawFd;
    use std::os::unix::process::CommandExt;

    #[test]
    fn terminated_descendants_do_not_make_the_next_operation_busy() {
        for _ in 0..8 {
            let common = tempfile::tempdir().unwrap();
            let file = common.path().join("repo-batch.lock");
            std::fs::File::create(&file).unwrap();
            let mut leader = std::process::Command::new("/bin/sleep")
                .arg("30")
                .process_group(0)
                .spawn()
                .unwrap();
            let group = leader.id() as libc::pid_t;
            let held = std::fs::File::open(&file).unwrap();
            assert_eq!(unsafe { libc::flock(held.as_raw_fd(), libc::LOCK_EX) }, 0);
            struct Children(Vec<libc::pid_t>);
            impl Drop for Children {
                fn drop(&mut self) {
                    for &pid in &self.0 {
                        unsafe {
                            libc::kill(pid, libc::SIGKILL);
                            libc::waitpid(pid, std::ptr::null_mut(), 0);
                        }
                    }
                }
            }
            let mut children = Children(vec![]);
            for _ in 0..16 {
                let child = unsafe { libc::fork() };
                assert!(child >= 0);
                if child == 0 {
                    unsafe {
                        if libc::setpgid(0, group) != 0 {
                            libc::_exit(1);
                        }
                        loop {
                            libc::pause();
                        }
                    }
                }
                children.0.push(child);
                assert_eq!(unsafe { libc::setpgid(child, group) }, 0);
            }
            drop(held);
            leader.kill().unwrap();
            leader.wait().unwrap();
            stop_group(&mut leader).unwrap();
            let _next =
                acquire(common.path()).expect("shutdown must not make the next operation busy");
        }
    }
}
