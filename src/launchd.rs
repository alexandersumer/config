//! The managed launchd agent that holds local Docker images to their budget.
use crate::error::Result;
use std::fs;
use std::path::{Path, PathBuf};

const LABEL: &str = "io.github.alexandersumer.workctl.docker-image-clean";
/// A fresh build has two hours to start its container before it can be removed.
const ARGUMENTS: [&str; 10] = [
    "docker",
    "image",
    "clean",
    "--max-used-space",
    "80GB",
    "--older-than",
    "2h",
    "--apply",
    "--heartbeat",
    "--json",
];
/// Three missed hourly runs are a stopped schedule, not a slow one.
const OVERDUE_SECONDS: u64 = 3 * 3600;
/// After a restart the agent gets its first hourly turn before it can be overdue.
const SETTLE_SECONDS: u64 = 90 * 60;

fn plist_path(home: &Path) -> PathBuf {
    home.join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist"))
}

fn render(home: &Path) -> Result<String> {
    let home = home
        .to_str()
        .ok_or("launch agent requires a UTF-8 home directory")?
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let arguments: String = ARGUMENTS
        .iter()
        .map(|argument| format!("    <string>{argument}</string>\n"))
        .collect();
    // launchd starts agents with a minimal PATH that omits every Docker CLI location.
    // Calendar times, unlike intervals, also fire once on wake for the hours slept through.
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{home}/.local/bin/workctl</string>
{arguments}  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PATH</key>
    <string>{home}/.docker/bin:/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin</string>
  </dict>
  <key>StartCalendarInterval</key>
  <dict>
    <key>Minute</key>
    <integer>0</integer>
  </dict>
  <key>ProcessType</key>
  <string>Background</string>
  <key>StandardOutPath</key>
  <string>{home}/Library/Logs/workctl-docker-image-clean.log</string>
</dict>
</plist>
"#
    ))
}

/// launchd loads agents only from the account's own home directory.
fn account_home(home: &Path) -> bool {
    let account = unsafe { libc::getpwuid(libc::getuid()) };
    if account.is_null() {
        return false;
    }
    use std::os::unix::ffi::OsStrExt;
    let directory = unsafe { std::ffi::CStr::from_ptr((*account).pw_dir) };
    let directory = Path::new(std::ffi::OsStr::from_bytes(directory.to_bytes()));
    home.canonicalize()
        .ok()
        .zip(directory.canonicalize().ok())
        .is_some_and(|(home, directory)| home == directory)
}

fn launchctl(arguments: &[&std::ffi::OsStr]) -> Result<std::process::Output> {
    std::process::Command::new("/bin/launchctl")
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot run launchctl: {error}"))
}

fn service() -> (String, String) {
    let domain = format!("gui/{}", unsafe { libc::getuid() });
    let service = format!("{domain}/{LABEL}");
    (domain, service)
}

fn loaded() -> Result<bool> {
    let (_, service) = service();
    Ok(launchctl(&["print".as_ref(), service.as_ref()])?
        .status
        .success())
}

/// Load the agent now; without a login session it loads at the next login.
fn load(plist: &Path, changed: bool) -> Result<()> {
    let (domain, service) = service();
    if loaded()? {
        if !changed {
            return Ok(());
        }
        launchctl(&["bootout".as_ref(), service.as_ref()])?;
        // Removal finishes asynchronously, and loading over it fails.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while loaded()? && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
    let output = launchctl(&["bootstrap".as_ref(), domain.as_ref(), plist.as_os_str()])?;
    if output.status.success() {
        println!("Loaded launch agent -> {LABEL}");
    } else {
        println!(
            "Launch agent loads at next login ({})",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// Whether the agent file must be written. Anything but a regular file at its
/// path is foreign and blocks installation before any managed file changes.
pub(crate) fn pending(home: &Path) -> Result<bool> {
    if !cfg!(target_os = "macos") {
        return Ok(false);
    }
    let target = plist_path(home);
    match fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.is_file() => Ok(fs::read(&target)
            .map_err(|error| format!("{}: {error}", target.display()))?
            != render(home)?.as_bytes()),
        Ok(_) => Err(format!(
            "{}: launch agent is not a regular file; preserve it and resolve manually",
            target.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(format!("{}: {error}", target.display())),
    }
}

pub(crate) fn install(home: &Path) -> Result<()> {
    if !cfg!(target_os = "macos") {
        return Ok(());
    }
    let target = plist_path(home);
    let expected = render(home)?;
    let changed = pending(home)?;
    if changed {
        use std::io::Write;
        let directory = target.parent().expect("launch agent directory");
        fs::create_dir_all(directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        let mut staged =
            tempfile::NamedTempFile::new_in(directory).map_err(|error| error.to_string())?;
        staged
            .write_all(expected.as_bytes())
            .and_then(|_| staged.as_file().sync_all())
            .map_err(|error| error.to_string())?;
        // launchd refuses agents that other accounts can write.
        {
            use std::os::unix::fs::PermissionsExt;
            staged
                .as_file()
                .set_permissions(fs::Permissions::from_mode(0o644))
                .map_err(|error| error.to_string())?;
        }
        staged.persist(&target).map_err(|error| error.to_string())?;
        println!("Installed launch agent -> {}", target.display());
    }
    if account_home(home) {
        load(&target, changed)?;
    }
    Ok(())
}

/// Read-only drift report for `workctl doctor --managed`.
pub(crate) fn diagnostics(home: &Path) -> Result<Vec<String>> {
    if !cfg!(target_os = "macos") {
        return Ok(Vec::new());
    }
    let target = plist_path(home);
    if fs::read(&target).ok().as_deref() != Some(render(home)?.as_bytes()) {
        return Ok(vec![
            "managed Docker image cleanup launch agent is missing or differs from this executable"
                .into(),
        ]);
    }
    if account_home(home) {
        if !loaded()? {
            return Ok(vec![
                "managed Docker image cleanup launch agent is not loaded".into(),
            ]);
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        if let Some(problem) = overdue(now, crate::docker::last_success(), booted()) {
            return Ok(vec![problem]);
        }
    }
    Ok(Vec::new())
}

/// The schedule is overdue once the machine has been up long enough for a run
/// and none has succeeded recently. An unknown boot time counts as long ago.
fn overdue(now: u64, last_success: Option<u64>, booted: Option<u64>) -> Option<String> {
    let since = |earlier: u64| now.saturating_sub(earlier);
    if booted.is_some_and(|booted| since(booted) <= SETTLE_SECONDS)
        || last_success.is_some_and(|succeeded| since(succeeded) <= OVERDUE_SECONDS)
    {
        return None;
    }
    Some(format!(
        "scheduled Docker image cleanup {}; see ~/Library/Logs/workctl-docker-image-clean.log",
        last_success.map_or_else(
            || "has never succeeded".to_owned(),
            |succeeded| format!(
                "last succeeded {} ago",
                crate::presentation::duration(std::time::Duration::from_secs(since(succeeded)))
            )
        )
    ))
}

#[cfg(target_os = "macos")]
fn booted() -> Option<u64> {
    let mut time: libc::timeval = unsafe { std::mem::zeroed() };
    let mut size = std::mem::size_of_val(&time);
    let read = unsafe {
        libc::sysctlbyname(
            c"kern.boottime".as_ptr(),
            (&mut time as *mut libc::timeval).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    (read == 0 && time.tv_sec > 0).then_some(time.tv_sec as u64)
}

#[cfg(not(target_os = "macos"))]
fn booted() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_runs_the_installed_workctl_hourly_with_a_docker_path_and_escaped_home() {
        let plist = render(Path::new("/Users/a&b")).unwrap();
        for expected in [
            "<string>io.github.alexandersumer.workctl.docker-image-clean</string>",
            "<string>/Users/a&amp;b/.local/bin/workctl</string>\n    <string>docker</string>\n    <string>image</string>\n    <string>clean</string>\n    <string>--max-used-space</string>\n    <string>80GB</string>\n    <string>--older-than</string>\n    <string>2h</string>\n    <string>--apply</string>\n    <string>--heartbeat</string>\n    <string>--json</string>\n  </array>",
            "<string>/Users/a&amp;b/.docker/bin:/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin</string>",
            "<key>StartCalendarInterval</key>\n  <dict>\n    <key>Minute</key>\n    <integer>0</integer>\n  </dict>",
            "<string>/Users/a&amp;b/Library/Logs/workctl-docker-image-clean.log</string>",
        ] {
            assert!(plist.contains(expected), "{expected}\n{plist}");
        }
        assert!(!plist.contains("a&b"));
        // Interval timers skip every firing that falls while the machine sleeps.
        assert!(!plist.contains("StartInterval"));
        assert!(!plist.contains("RunAtLoad"));
    }

    #[test]
    fn schedule_is_overdue_only_after_it_had_a_turn_and_no_recent_success() {
        let now = 1_000_000;
        let hours = |count: u64| Some(now - count * 3600);
        assert_eq!(overdue(now, hours(1), hours(100)), None);
        assert_eq!(overdue(now, hours(3), hours(100)), None);
        assert_eq!(overdue(now, hours(50), hours(1)), None);
        assert_eq!(overdue(now, None, hours(1)), None);
        assert!(overdue(now, hours(4), hours(100))
            .unwrap()
            .contains("last succeeded 4h 0m 0s ago"));
        assert!(overdue(now, None, hours(100))
            .unwrap()
            .contains("has never succeeded"));
        assert!(overdue(now, hours(4), None).is_some());
    }
}
