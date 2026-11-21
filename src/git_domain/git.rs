//! The installed Git executable is the authority for repository facts.
use super::display;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{Seek, Write};
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

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

pub(super) fn bitbucket_ssh_moved(error: &str) -> bool {
    error.contains("We have moved our SSH host to")
        && error.contains("ssh.bitbucket.org")
        && error.contains("The old host bitbucket.org")
}

fn bitbucket_ssh_path(url: &str) -> Option<&str> {
    url.strip_prefix("git@bitbucket.org:")
        .or_else(|| url.strip_prefix("ssh://git@bitbucket.org/"))
}

pub(super) fn remote_access_error(error: &str) -> bool {
    static ACCESS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    ACCESS.get_or_init(|| regex::Regex::new(r"(?i)You may not have access to this repository|repository[^\r\n]*not found|does not appear to be a git repository|Authentication failed|Permission denied \((?:publickey|keyboard-interactive|password)|could not read (?:Username|Password)|(?:returned error:|HTTP/[\d.]+)\s*(?:401|403)\b").unwrap()).is_match(error)
}

pub(super) fn command(path: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(path)
        // Safety decisions must inspect the filesystem even when a monitor's
        // cached change list is stale. Apply this to native removal/reset too.
        .args(["-c", "core.fsmonitor=false"])
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
    capture_with_deadline(
        path,
        args,
        probe.then(|| Instant::now() + Duration::from_secs(15)),
    )
}

fn remaining(deadline: Option<Instant>) -> Result<Duration, String> {
    deadline.map_or(Ok(Duration::from_secs(15)), |end| {
        end.checked_duration_since(Instant::now())
            .ok_or_else(|| "Checkout verification deadline expired".into())
    })
}

fn capture_with_deadline(
    path: &Path,
    args: &[&OsStr],
    deadline: Option<Instant>,
) -> Result<(i32, Vec<u8>, Vec<u8>), String> {
    let mut cmd = command(path);
    cmd.args(args).stdin(Stdio::null());
    crate::runtime::capture(cmd, remaining(deadline)?, deadline.is_some())
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

fn anonymous_input(bytes: &[u8]) -> Result<fs::File, String> {
    let mut input = tempfile::tempfile().map_err(|e| e.to_string())?;
    input
        .write_all(bytes)
        .and_then(|_| input.rewind())
        .map_err(|e| e.to_string())?;
    Ok(input)
}

#[derive(Clone, Copy)]
pub(super) struct FetchIo<'a> {
    pub stderr: &'a fs::File,
    pub input: Option<&'a [u8]>,
}

pub(super) struct Git {
    pub cwd: PathBuf,
    pub directory: PathBuf,
    deadline: Option<Instant>,
}
impl Git {
    pub fn new(cwd: PathBuf) -> Result<Self, String> {
        Self::open(cwd, None)
    }
    pub fn for_verification(cwd: PathBuf) -> Result<Self, String> {
        Self::open(cwd, Some(Instant::now() + Duration::from_secs(5)))
    }
    fn open(cwd: PathBuf, deadline: Option<Instant>) -> Result<Self, String> {
        let (code, out, err) = capture_with_deadline(
            &cwd,
            &[OsStr::new("rev-parse"), OsStr::new("--absolute-git-dir")],
            deadline,
        )?;
        if code != 0 {
            return Err(redact(OsString::from_vec(err)));
        }
        let directory = path_output(&out)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            cwd,
            directory,
            deadline,
        })
    }
    pub fn call(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        let args: Vec<_> = args.iter().map(OsStr::new).collect();
        self.call_os(&args)
    }
    pub fn call_os(&self, args: &[&OsStr]) -> Result<Vec<u8>, String> {
        self.call_captured(args, None)
    }
    fn call_captured(
        &self,
        args: &[&OsStr],
        fetch: Option<FetchIo<'_>>,
    ) -> Result<Vec<u8>, String> {
        let (code, out, err) = if let Some(fetch) = fetch {
            let mut cmd = command(&self.cwd);
            cmd.args(args);
            let stdin = fetch
                .input
                .map(anonymous_input)
                .transpose()?
                .map_or_else(Stdio::null, Stdio::from);
            crate::runtime::capture_reset_fetch(cmd, fetch.stderr, stdin)?
        } else {
            capture_with_deadline(&self.cwd, args, self.deadline)?
        };
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
    /// Git's non-fetching object lookup. Older Git releases that do not
    /// recognize the global option cannot establish local availability.
    pub fn has_local_commit(&self, oid: &str) -> bool {
        let expected_format = if oid.len() == 40 { "sha1" } else { "sha256" };
        self.text(&["rev-parse", "--show-object-format"])
            .is_ok_and(|format| format == expected_format)
            && self
                .optional(&[
                    "--no-lazy-fetch",
                    "--no-replace-objects",
                    "cat-file",
                    "-t",
                    oid,
                ])
                .is_ok_and(|output| output.as_deref() == Some(b"commit\n"))
    }
    pub fn sparse_included(&self, paths: &[u8]) -> Result<Vec<u8>, String> {
        let input = anonymous_input(paths)?;
        let mut command = command(&self.cwd);
        command.args(["sparse-checkout", "check-rules", "-z"]);
        let (code, included, error) = crate::runtime::capture_with_input(
            command,
            remaining(self.deadline)?,
            self.deadline.is_some(),
            input,
        )?;
        if code != 0 {
            return Err(format!(
                "Cannot verify sparse checkout rules: {}",
                diagnostic(OsString::from_vec(error))
            ));
        }
        Ok(included)
    }
    pub fn missing_blobs(&self, objects: &[u8]) -> Result<Vec<u8>, String> {
        let mut cmd = command(&self.cwd);
        cmd.args([
            "--no-lazy-fetch",
            "--no-replace-objects",
            "cat-file",
            "--batch-check=%(objecttype)",
        ]);
        let (code, out, err) = crate::runtime::capture_with_input(
            cmd,
            remaining(self.deadline)?,
            self.deadline.is_some(),
            anonymous_input(objects)?,
        )?;
        if code != 0 {
            return Err(format!(
                "Cannot check local checkout objects: {}",
                diagnostic(OsString::from_vec(err))
            ));
        }
        let lines = |bytes| std::str::from_utf8(bytes).map(str::lines);
        let wants = lines(objects).map_err(|_| "Invalid Git object input")?;
        let mut results = lines(&out).map_err(|_| "Invalid Git object check response")?;
        let mut missing = Vec::new();
        for oid in wants {
            match results.next() {
                Some("blob") => {}
                Some(line) if line.strip_suffix(" missing") == Some(oid) => {
                    missing.extend_from_slice(oid.as_bytes());
                    missing.push(b'\n');
                }
                _ => return Err("Unexpected Git object check response".into()),
            }
        }
        if results.next().is_some() {
            return Err("Unexpected trailing Git object check results".into());
        }
        Ok(missing)
    }

    pub fn remote_call(
        &self,
        remote: &str,
        effective_url: &str,
        redirect: &mut Option<String>,
        args: &[&str],
        fetch: Option<FetchIo<'_>>,
        before_retry: impl FnOnce() -> Result<(), String>,
    ) -> Result<Vec<u8>, String> {
        let call = |args: &[&str]| {
            let args: Vec<_> = args.iter().map(OsStr::new).collect();
            self.call_captured(&args, fetch)
        };
        let redirected = |rewrite: &str| {
            let args: Vec<_> = ["-c", rewrite]
                .into_iter()
                .chain(args.iter().copied())
                .collect();
            call(&args)
        };
        if let Some(rewrite) = redirect {
            return redirected(rewrite);
        }
        let error = match call(args) {
            Ok(output) => return Ok(output),
            Err(error) if bitbucket_ssh_moved(&error) => error,
            Err(error) => return Err(error),
        };

        // Git applies insteadOf once. Match the stored URL (including HTTPS
        // when it was rewritten to legacy SSH) and confirm our rule won.
        let Some(path) = bitbucket_ssh_path(effective_url) else {
            return Err(error);
        };
        let Some((workspace, repository)) = path.split_once('/') else {
            return Err(error);
        };
        let plain_component = |component: &str| {
            !component.is_empty()
                && component != "."
                && component != ".."
                && component
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
        };
        if !plain_component(workspace) || !plain_component(repository) {
            return Err(error);
        }
        let Ok(Some(urls)) = self.optional(&[
            "config",
            "--null",
            "--get-all",
            &format!("remote.{remote}.url"),
        ]) else {
            return Err(error);
        };
        let Some(raw) = urls
            .split(|byte| *byte == 0)
            .next()
            .and_then(|raw| std::str::from_utf8(raw).ok())
        else {
            return Err(error);
        };
        let raw_path =
            bitbucket_ssh_path(raw).or_else(|| raw.strip_prefix("https://bitbucket.org/"));
        if raw_path != Some(path) {
            return Err(error);
        }
        let migrated = format!("git@ssh.bitbucket.org:{path}");
        let rewrite = format!("url.{migrated}.insteadOf={raw}");
        if self
            .text(&["-c", &rewrite, "remote", "get-url", remote])
            .ok()
            .as_deref()
            != Some(migrated.as_str())
        {
            return Err(error);
        }
        eprintln!(
            "Bitbucket retired the old SSH endpoint; retrying this remote at ssh.bitbucket.org."
        );
        before_retry()?;
        *redirect = Some(rewrite);
        redirected(redirect.as_deref().unwrap()).map_err(|error| {
            format!("{error}\nBitbucket SSH migration redirected this request to ssh.bitbucket.org; verify access and the SSH host key for that endpoint.")
        })
    }
    pub fn action(&self, args: &[&str]) -> Result<(), String> {
        let output = self.call(args)?;
        print!("{}", redact(OsString::from_vec(output)));
        Ok(())
    }
    pub fn optional(&self, args: &[&str]) -> Result<Option<Vec<u8>>, String> {
        let args: Vec<_> = args.iter().map(OsStr::new).collect();
        let (code, out, err) = capture_with_deadline(&self.cwd, &args, self.deadline)?;
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
        reference_text(self.call(args)?)
    }
    pub fn metadata(&self, name: &str) -> PathBuf {
        // All worker targets are primary checkouts. These operation markers and
        // FETCH_HEAD belong to their Git directory, resolved once per attempt.
        self.directory.join(name)
    }
}

pub(super) fn reference_text(bytes: Vec<u8>) -> Result<String, String> {
    String::from_utf8(bytes)
        .map(|s| s.trim_end_matches('\n').to_owned())
        .map_err(|_| "Git returned non-UTF-8 reference text".into())
}

pub(super) fn exists(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("{}: {e}", display(path))),
    }
}
