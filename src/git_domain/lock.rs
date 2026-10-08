//! Common-directory coordination belongs to the Git domain.
use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
pub(super) fn acquire(common: &Path) -> Result<File, String> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(common.join("repo-batch.lock"))
        .map_err(|e| e.to_string())?;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(format!(
            "Repository is busy or cannot be locked: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(file)
}
