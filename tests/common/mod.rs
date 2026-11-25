//! Helpers shared by the workctl integration suites.
#![allow(dead_code)] // Each suite uses the helpers it needs.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

/// Carries the pace into shims and fixture scripts.
pub const PACE: &str = "WORKCTL_TEST_PACE";

/// How many times slower than an idle machine this one starts Git right now.
///
/// Every time budget in these suites is written for an idle machine. A busy
/// one starts each subprocess late, which must not read as a hang or a missed
/// deadline. Multiplying every wait, product deadline, and simulated hang in a
/// scenario by the same pace keeps its proportions, so an idle machine runs
/// the written values and a busy one runs the same scenario more slowly.
///
/// The pace is measured again as a suite runs and only ever rises, so a budget
/// taken later in a test is never tighter than one taken earlier.
pub fn pace() -> u64 {
    static PACE: Mutex<(u64, Option<Instant>)> = Mutex::new((1, None));
    let mut pace = PACE.lock().unwrap_or_else(PoisonError::into_inner);
    if pace
        .1
        .is_none_or(|measured| measured.elapsed() >= Duration::from_secs(10))
    {
        // The median of five launches, against a 25 ms launch on an idle machine.
        // Budgets already allow for the occasional slow launch; an outlier here
        // would stretch every deadline in the suite.
        let mut launches: Vec<_> = (0..5)
            .map(|_| {
                let started = Instant::now();
                let _ = Command::new("git")
                    .arg("--version")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                started.elapsed()
            })
            .collect();
        launches.sort();
        pace.0 = pace
            .0
            .max((launches[2].as_millis() as u64 / 25).clamp(1, 30));
        pace.1 = Some(Instant::now());
    }
    pace.0
}

/// Seconds for a product deadline or simulated hang at the current pace.
pub fn secs(idle: u64) -> u64 {
    idle * pace()
}

/// A wait or elapsed-time bound at the current pace.
pub fn time(idle: u64) -> Duration {
    Duration::from_secs(secs(idle))
}

/// Shell for the top of a fixture script. `$PACE` scales polling limits, and
/// whole-second sleeps are scenario durations that follow it; fractional
/// sleeps are polling and do not.
pub const PACED_SHELL: &str = r#"PACE=${WORKCTL_TEST_PACE:-1}
sleep() { case "$1" in *[!0-9]*) command sleep "$1";; *) command sleep "$(($1 * PACE))";; esac; }"#;

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
