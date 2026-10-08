//! The installed Git executable is the authority for repository facts.
use super::{display, supervisor};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{Read, Seek};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub(super) fn command(path: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(path)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "Never")
        .env("LC_ALL", "C");
    // Git's hook environment must not redirect operations into another checkout.
    for name in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_PREFIX",
        "GIT_NAMESPACE",
        "GIT_SHALLOW_FILE",
        "GIT_IMPLICIT_WORK_TREE",
    ] {
        cmd.env_remove(name);
    }
    cmd
}

pub(super) fn capture(
    path: &Path,
    args: &[&OsStr],
    probe: bool,
) -> Result<(i32, Vec<u8>, Vec<u8>), String> {
    let mut cmd = command(path);
    cmd.args(args).stdin(Stdio::null());
    // Discovery probes are bounded too, before the repository workers exist.
    let mut out = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut err = tempfile::tempfile().map_err(|e| e.to_string())?;
    cmd.stdout(out.try_clone().map_err(|e| e.to_string())?)
        .stderr(err.try_clone().map_err(|e| e.to_string())?);
    if probe {
        cmd.process_group(0);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot execute Git: {e}"))?;
    let deadline = Instant::now() + Duration::from_secs(15);
    let code = if !probe {
        child.wait().map_err(|e| e.to_string())?.code().unwrap_or(1)
    } else {
        loop {
            match supervisor::exited(&mut child) {
                Ok(true) => break supervisor::stop_group(&mut child)?.code().unwrap_or(1),
                Ok(false) => {}
                Err(error) => {
                    let _ = supervisor::stop_group(&mut child);
                    return Err(error);
                }
            }
            if supervisor::cancelled() || Instant::now() >= deadline {
                supervisor::stop_group(&mut child)?;
                return Err("Git discovery probe interrupted or timed out".into());
            }
            thread::sleep(Duration::from_millis(2));
        }
    };
    let mut stdout = vec![];
    let mut stderr = vec![];
    out.rewind()
        .and_then(|_| out.read_to_end(&mut stdout))
        .map_err(|e| e.to_string())?;
    err.rewind()
        .and_then(|_| err.read_to_end(&mut stderr))
        .map_err(|e| e.to_string())?;
    Ok((code, stdout, stderr))
}

pub(super) fn probe(path: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let args: Vec<_> = args.iter().map(OsStr::new).collect();
    let (code, out, err) = capture(path, &args, true)?;
    if code == 0 {
        Ok(out)
    } else {
        Err(format!(
            "{}: {}",
            display(path),
            display(OsString::from_vec(err))
        ))
    }
}

pub(super) fn path_output(bytes: &[u8]) -> PathBuf {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    PathBuf::from(OsString::from_vec(bytes.to_vec()))
}

pub(super) struct Git {
    pub cwd: PathBuf,
    pub directory: PathBuf,
}
impl Git {
    pub fn new(cwd: PathBuf) -> Result<Self, String> {
        let (code, out, err) = capture(
            &cwd,
            &[OsStr::new("rev-parse"), OsStr::new("--absolute-git-dir")],
            false,
        )?;
        if code != 0 {
            return Err(String::from_utf8_lossy(&err).into_owned());
        }
        let directory = path_output(&out)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        Ok(Self { cwd, directory })
    }
    pub fn call(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        let args: Vec<_> = args.iter().map(OsStr::new).collect();
        let (code, out, err) = capture(&self.cwd, &args, false)?;
        // Logs contain stderr from successful hooks as well as failed commands.
        if !err.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&err));
        }
        if code != 0 {
            return Err(format!(
                "git {} failed (exit {code}): {}",
                args.iter().map(display).collect::<Vec<_>>().join(" "),
                String::from_utf8_lossy(&err).trim()
            ));
        }
        Ok(out)
    }
    pub fn action(&self, args: &[&str]) -> Result<(), String> {
        let output = self.call(args)?;
        print!("{}", String::from_utf8_lossy(&output));
        Ok(())
    }
    pub fn optional(&self, args: &[&str]) -> Result<Option<Vec<u8>>, String> {
        let args: Vec<_> = args.iter().map(OsStr::new).collect();
        let (code, out, err) = capture(&self.cwd, &args, false)?;
        match code {
            0 => Ok(Some(out)),
            1 => Ok(None),
            _ => Err(format!(
                "Git probe failed: {}",
                String::from_utf8_lossy(&err)
            )),
        }
    }
    pub fn text(&self, args: &[&str]) -> Result<String, String> {
        String::from_utf8(self.call(args)?)
            .map(|s| s.trim_end_matches('\n').to_owned())
            .map_err(|_| "Git returned non-UTF-8 reference text".into())
    }
    pub fn metadata(&self, name: &str) -> PathBuf {
        // All worker targets are primary checkouts. These operation markers and
        // FETCH_HEAD belong to their Git directory, resolved once per attempt.
        self.directory.join(name)
    }
}

pub(super) fn exists(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("{}: {e}", display(path))),
    }
}
