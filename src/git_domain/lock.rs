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
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(format!(
            "Repository is busy or cannot be locked: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(file)
}
