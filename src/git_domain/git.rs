//! The installed Git executable is the authority for repository facts.
use super::display;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

pub(super) fn redact(value: impl AsRef<OsStr>) -> String {
    static URL: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let pattern = URL.get_or_init(|| {
        regex::Regex::new(r#"[A-Za-z][A-Za-z0-9+.-]*://[^\s"'<>]+"#).expect("static URL pattern")
    });
    pattern
        .replace_all(
            &value.as_ref().to_string_lossy(),
            |capture: &regex::Captures<'_>| {
                let url = &capture[0];
                let scheme = url.find("://").unwrap() + 3;
                let authority_end = url[scheme..]
                    .find(['/', '?', '#'])
                    .map_or(url.len(), |n| scheme + n);
                let url = if let Some(at) = url[scheme..authority_end].rfind('@') {
                    format!("{}[redacted]{}", &url[..scheme], &url[scheme + at..])
                } else {
                    url.to_owned()
                };
                if let Some((base, query)) = url.split_once('?') {
                    let query = query
                        .split('&')
                        .map(|parameter| {
                            parameter.split_once('=').map_or_else(
                                || parameter.to_owned(),
                                |(key, _)| format!("{key}=[redacted]"),
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("&");
                    format!("{base}?{query}")
                } else {
                    url
                }
            },
        )
        .into_owned()
}

pub(super) fn diagnostic(value: impl AsRef<OsStr>) -> String {
    display(redact(value))
}

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
    crate::runtime::capture(cmd, Duration::from_secs(15), probe)
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
            diagnostic(OsString::from_vec(err))
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
            return Err(redact(OsString::from_vec(err)));
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
            eprint!("{}", redact(OsString::from_vec(err.clone())));
        }
        if code != 0 {
            return Err(format!(
                "git {} failed (exit {code}): {}",
                args.iter().map(diagnostic).collect::<Vec<_>>().join(" "),
                redact(OsString::from_vec(err)).trim()
            ));
        }
        Ok(out)
    }
    pub fn action(&self, args: &[&str]) -> Result<(), String> {
        let output = self.call(args)?;
        print!("{}", redact(OsString::from_vec(output)));
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
                redact(OsString::from_vec(err))
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
