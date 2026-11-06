//! Helpers shared by the workctl integration suites.
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Private run-history root for this test process.
///
/// Tests must never write to the user's `~/.local/state/workctl`, and concurrent
/// suites must not contend on one shared history directory. workctl creates the
/// directory itself with the 0700 mode it requires. State left by test processes
/// that have since exited is removed, so nothing accumulates across runs.
pub fn state_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let parent = Path::new(env!("CARGO_TARGET_TMPDIR"));
        for entry in fs::read_dir(parent).into_iter().flatten().flatten() {
            let pid = entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_prefix("workctl-state-"))
                .and_then(|pid| pid.parse::<libc::pid_t>().ok());
            if pid.is_some_and(|pid| unsafe { libc::kill(pid, 0) } != 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)) {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
        let dir = parent.join(format!("workctl-state-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    })
}
