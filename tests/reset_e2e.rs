//! Network E2E: standalone CLI, installed Git, real git-daemon, and persisted state.
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::net::TcpListener;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

struct Runtime {
    temp: Option<TempDir>,
    daemon: Option<Child>,
    binary: PathBuf,
    artifacts: PathBuf,
}
impl Runtime {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("create E2E workspace");
        let binary = temp.path().join("workctl");
        fs::copy(env!("CARGO_BIN_EXE_workctl"), &binary).expect("copy standalone executable");
        let artifacts = temp.path().join("artifacts");
        fs::create_dir(&artifacts).unwrap();
        Self {
            temp: Some(temp),
            daemon: None,
            binary,
            artifacts,
        }
    }
    fn root(&self) -> &Path {
        self.temp.as_ref().unwrap().path()
    }
    fn command(&self, path: &Path, program: impl AsRef<Path>) -> Command {
        let mut cmd = Command::new(program.as_ref());
        cmd.current_dir(path)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_AUTHOR_NAME", "E2E")
            .env("GIT_AUTHOR_EMAIL", "e2e@example.com")
            .env("GIT_COMMITTER_NAME", "E2E")
            .env("GIT_COMMITTER_EMAIL", "e2e@example.com")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("NO_COLOR", "1")
            .env("TMPDIR", &self.artifacts)
            .stdin(Stdio::null());
        for name in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_COMMON_DIR",
            "GIT_CONFIG_COUNT",
        ] {
            cmd.env_remove(name);
        }
        cmd
    }
    fn run(&self, mut cmd: Command) -> Output {
        let mut transcript = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.artifacts.join("commands.log"))
            .unwrap();
        writeln!(transcript, "Command: {cmd:?}").unwrap();
        let out = tempfile::tempfile().unwrap();
        let err = tempfile::tempfile().unwrap();
        cmd.stdout(out.try_clone().unwrap())
            .stderr(err.try_clone().unwrap())
            .process_group(0);
        let mut child = cmd
            .spawn()
            .expect("required E2E executable must be available");
        let deadline = Instant::now() + Duration::from_secs(20);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.wait();
                panic!("E2E command deadline exceeded: {cmd:?}");
            }
            thread::sleep(Duration::from_millis(10));
        };
        // The command is reaped before its group is cleaned, as in the product.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        use std::io::{Read, Seek};
        let mut out = out;
        let mut err = err;
        out.rewind().unwrap();
        err.rewind().unwrap();
        let mut stdout = vec![];
        let mut stderr = vec![];
        out.read_to_end(&mut stdout).unwrap();
        err.read_to_end(&mut stderr).unwrap();
        writeln!(
            transcript,
            "Exit: {status}\n{}{}",
            String::from_utf8_lossy(&stdout),
            String::from_utf8_lossy(&stderr)
        )
        .unwrap();
        Output {
            status,
            stdout,
            stderr,
        }
    }
    fn git(&self, path: &Path, args: &[&str]) -> String {
        let mut cmd = self.command(path, "git");
        cmd.args(args);
        let out = self.run(cmd);
        assert!(out.status.success(), "git {args:?}: {}", text(&out));
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    }
    fn reset(&self, path: &Path, args: &[&str], code: i32) -> Output {
        let mut cmd = self.command(path, &self.binary);
        cmd.args(["git", "reset", "--keep-logs"]).args(args);
        let out = self.run(cmd);
        assert_eq!(out.status.code(), Some(code), "{}", text(&out));
        out
    }
    fn cleanup(&self, args: &[&str], code: i32) -> serde_json::Value {
        let mut cmd = self.command(self.root(), &self.binary);
        cmd.args(["git", "worktree", "clean", "--json"]).args(args);
        let out = self.run(cmd);
        assert_eq!(out.status.code(), Some(code), "{}", text(&out));
        assert!(
            !out.stdout.contains(&0x1b),
            "JSON must not contain terminal decoration"
        );
        let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result["schema_version"], 2);
        result
    }
    fn serve(&mut self, origin: &Path) -> String {
        let port = TcpListener::bind("127.0.0.1:0")
            .expect("loopback networking required")
            .local_addr()
            .unwrap()
            .port();
        let log = File::create(self.artifacts.join("git-daemon.log")).unwrap();
        let mut cmd = self.command(self.root(), "git");
        cmd.args([
            "daemon",
            "--verbose",
            "--export-all",
            "--strict-paths",
            "--listen=127.0.0.1",
        ])
        .arg(format!("--port={port}"))
        .arg(format!("--base-path={}", self.root().display()))
        .arg(origin)
        .stdout(log.try_clone().unwrap())
        .stderr(log)
        .process_group(0);
        self.daemon = Some(cmd.spawn().expect("git daemon is required, not optional"));
        let url = format!("git://127.0.0.1:{port}/origin.git");
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let mut cmd = self.command(self.root(), "git");
            cmd.args(["ls-remote", "--symref", &url, "HEAD"]);
            let out = self.run(cmd);
            if out.status.success() {
                assert!(text(&out).contains("ref: refs/heads/trunk\tHEAD"));
                return url;
            }
            assert!(
                self.daemon.as_mut().unwrap().try_wait().unwrap().is_none(),
                "Git daemon exited; inspect git-daemon.log"
            );
            assert!(
                Instant::now() < deadline,
                "Git protocol readiness failed: {}",
                text(&out)
            );
            thread::sleep(Duration::from_millis(20));
        }
    }
}

#[test]
fn standalone_cleanup_uses_current_git_protocol_evidence_and_explicit_discard() {
    let mut rt = Runtime::new();
    let seed = rt.root().join("seed");
    fs::create_dir(&seed).unwrap();
    rt.git(&seed, &["init", "-b", "trunk"]);
    fs::write(seed.join("file"), "published\n").unwrap();
    rt.git(&seed, &["add", "."]);
    rt.git(&seed, &["commit", "-m", "published baseline"]);
    let baseline = rt.git(&seed, &["rev-parse", "HEAD"]);
    let origin = rt.root().join("origin.git");
    rt.git(rt.root(), &["clone", "--bare", path(&seed), path(&origin)]);
    let url = rt.serve(&origin);
    let primary = rt.root().join("primary checkout");
    rt.git(rt.root(), &["clone", &url, path(&primary)]);
    let published = rt.root().join("published linked worktree");
    let private = rt.root().join("private linked worktree");
    rt.git(
        &primary,
        &["worktree", "add", "-b", "published", path(&published)],
    );
    rt.git(
        &primary,
        &["worktree", "add", "-b", "private", path(&private)],
    );
    fs::write(private.join("private-commit"), "unpublished work\n").unwrap();
    rt.git(&private, &["add", "."]);
    rt.git(&private, &["commit", "-m", "private commit"]);
    let private_head = rt.git(&private, &["rev-parse", "HEAD"]);
    let registrations = rt.git(&primary, &["worktree", "list", "--porcelain"]);

    let preview = rt.cleanup(&[path(&primary)], 1);
    assert_eq!(preview["status"], "blocked");
    assert_eq!(preview["summary"]["would_remove"], 1);
    assert_eq!(preview["summary"]["blocked"], 1);
    assert_eq!(
        rt.git(&primary, &["worktree", "list", "--porcelain"]),
        registrations
    );
    assert!(
        published.is_dir() && private.is_dir(),
        "preview must not delete files"
    );

    let partial = rt.cleanup(&["--apply", path(&primary)], 1);
    assert_eq!(
        partial["status"], "blocked",
        "partial cleanup must not claim success"
    );
    assert_eq!(partial["summary"]["removed"], 1);
    assert_eq!(partial["summary"]["blocked"], 1);
    assert!(!published.exists() && private.is_dir());
    assert!(!rt
        .git(&primary, &["worktree", "list", "--porcelain"])
        .contains(path(&published)));
    assert_eq!(rt.git(&private, &["rev-parse", "HEAD"]), private_head);

    // Publish and cache the branch, then delete it remotely. The cached ref must
    // remain, so a cleanup that trusts it instead of current evidence is caught.
    rt.git(&private, &["push", path(&origin), "private:private"]);
    rt.git(&primary, &["fetch", "origin"]);
    rt.git(&primary, &["push", path(&origin), ":private"]);
    assert_eq!(
        rt.git(&primary, &["rev-parse", "origin/private"]),
        private_head
    );
    let fetch_head = fs::read(primary.join(".git/FETCH_HEAD")).unwrap();
    let cached = rt.cleanup(&["--apply", path(&private)], 1);
    assert_eq!(cached["summary"]["blocked"], 1);
    assert!(cached["results"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("unpublished"));
    assert!(private.is_dir());
    assert_eq!(rt.git(&private, &["rev-parse", "HEAD"]), private_head);
    assert_eq!(
        rt.git(&primary, &["rev-parse", "origin/private"]),
        private_head
    );
    assert_eq!(
        fs::read(primary.join(".git/FETCH_HEAD")).unwrap(),
        fetch_head
    );

    rt.git(&private, &["push", path(&origin), "private:private"]);
    let local_file = private.join("precious local file");
    fs::write(&local_file, "preserve without explicit discard\n").unwrap();
    let local = rt.cleanup(&["--apply", path(&private)], 1);
    assert_eq!(local["summary"]["blocked"], 1);
    assert!(local["results"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("--discard-local"));
    assert_eq!(
        fs::read_to_string(&local_file).unwrap(),
        "preserve without explicit discard\n"
    );

    let removed = rt.cleanup(
        &["--apply", "--discard-local", path(&private), path(&private)],
        0,
    );
    assert_eq!(removed["status"], "completed");
    assert_eq!(removed["results"][0]["status"], "removed");
    assert_eq!(removed["summary"]["removed"], 1);
    assert!(!private.exists());
    let remaining = rt.git(&primary, &["worktree", "list", "--porcelain"]);
    assert_eq!(
        remaining
            .lines()
            .filter(|line| line.starts_with("worktree "))
            .count(),
        1
    );
    assert!(remaining.contains(path(&primary)));
    assert_eq!(rt.git(&primary, &["rev-parse", "HEAD"]), baseline);
    assert!(rt.git(&primary, &["status", "--porcelain"]).is_empty());
    let repeated = rt.cleanup(&["--apply", path(&primary)], 0);
    assert_eq!(repeated["status"], "completed");
    assert_eq!(repeated["summary"]["removed"], 0);
}
impl Drop for Runtime {
    fn drop(&mut self) {
        if let Some(mut daemon) = self.daemon.take() {
            unsafe {
                libc::kill(-(daemon.id() as i32), libc::SIGKILL);
            }
            let _ = daemon.wait();
        }
        if let Some(destination) = std::env::var_os("RESET_E2E_ARTIFACT_DIR") {
            let destination = PathBuf::from(destination).join(self.root().file_name().unwrap());
            for entry in walkdir::WalkDir::new(&self.artifacts) {
                let entry = entry.expect("read E2E artifacts");
                let target = destination.join(entry.path().strip_prefix(&self.artifacts).unwrap());
                if entry.file_type().is_dir() {
                    fs::create_dir_all(target).expect("preserve E2E artifacts");
                } else if entry.file_type().is_file() {
                    fs::copy(entry.path(), target).expect("preserve E2E artifact");
                }
            }
        } else if thread::panicking() {
            let retained = self.temp.take().unwrap().keep();
            eprintln!(
                "E2E daemon stopped; failure workspace retained: {}",
                retained.display()
            );
        }
    }
}
fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}
fn path(path: &Path) -> &str {
    path.to_str().unwrap()
}

#[test]
fn standalone_cli_resets_over_git_protocol_and_preserves_dirty_batch_member() {
    let mut rt = Runtime::new();
    let seed = rt.root().join("seed");
    fs::create_dir(&seed).unwrap();
    rt.git(&seed, &["init", "-b", "trunk"]);
    fs::write(seed.join("file"), "initial\n").unwrap();
    rt.git(&seed, &["add", "."]);
    rt.git(&seed, &["commit", "-m", "initial"]);
    let old = rt.git(&seed, &["rev-parse", "HEAD"]);
    let origin = rt.root().join("origin.git");
    rt.git(rt.root(), &["clone", "--bare", path(&seed), path(&origin)]);
    rt.git(&seed, &["remote", "add", "origin", path(&origin)]);
    let url = rt.serve(&origin);
    let single = rt.root().join("single");
    let first = rt.root().join("first");
    let second = rt.root().join("second");
    fs::create_dir(&first).unwrap();
    fs::create_dir(&second).unwrap();
    let alpha = first.join("alpha");
    let beta = second.join("beta");
    let dirty = second.join("dirty");
    for repo in [&single, &alpha, &beta, &dirty] {
        rt.git(rt.root(), &["clone", &url, path(repo)]);
    }
    rt.git(&single, &["branch", "feature"]);
    fs::write(single.join("local-commit"), "recover\n").unwrap();
    rt.git(&single, &["add", "."]);
    rt.git(&single, &["commit", "-m", "divergent local commit"]);
    let local = rt.git(&single, &["rev-parse", "HEAD"]);
    fs::write(single.join("scratch"), "preserve\n").unwrap();
    fs::write(seed.join("file"), "advanced\n").unwrap();
    rt.git(&seed, &["commit", "-am", "advance remote"]);
    rt.git(&seed, &["push", "origin", "trunk"]);
    let new = rt.git(&seed, &["rev-parse", "HEAD"]);

    rt.reset(&single, &["--version"], 0);
    let listing = rt.reset(rt.root(), &["--list", "first", "second"], 0);
    assert_eq!(String::from_utf8_lossy(&listing.stdout).lines().count(), 3);
    assert_eq!(
        rt.git(&alpha, &["rev-parse", "origin/trunk"]),
        old,
        "listing must not fetch"
    );
    let single_out = rt.reset(&single, &[], 0);
    assert!(text(&single_out).contains(&format!(
        "Reset {} to origin/trunk",
        single.canonicalize().unwrap().display()
    )));
    assert_eq!(rt.git(&single, &["rev-parse", "HEAD"]), new);
    assert_eq!(
        rt.git(&single, &["symbolic-ref", "--short", "HEAD"]),
        "trunk"
    );
    assert_eq!(rt.git(&single, &["rev-parse", "feature"]), old);
    assert_eq!(
        fs::read_to_string(single.join("scratch")).unwrap(),
        "preserve\n"
    );
    let refs = rt.git(
        &single,
        &[
            "for-each-ref",
            "--format=%(objectname)",
            "refs/home-reset-backups/",
        ],
    );
    assert!(
        refs.lines().any(|oid| oid == local),
        "divergent commit must remain recoverable"
    );

    fs::write(dirty.join("file"), "unsaved edits\n").unwrap();
    let config = fs::read(dirty.join(".git/config")).unwrap();
    let batch = rt.reset(rt.root(), &["first", "second"], 1);
    assert!(text(&batch).contains("Workers        3"));
    assert!(text(&batch).contains("2 succeeded\n  1 failed"));
    for repo in [&alpha, &beta] {
        assert_eq!(rt.git(repo, &["rev-parse", "HEAD"]), new);
    }
    assert_eq!(
        fs::read_to_string(dirty.join("file")).unwrap(),
        "unsaved edits\n",
        "dirty tracked files must not be overwritten"
    );
    assert_eq!(rt.git(&dirty, &["rev-parse", "HEAD"]), old);
    assert_eq!(
        rt.git(&dirty, &["rev-parse", "origin/trunk"]),
        old,
        "dirty refusal must precede network fetch"
    );
    assert_eq!(fs::read(dirty.join(".git/config")).unwrap(), config);
    fs::write(dirty.join("file"), "initial\n").unwrap();
    rt.reset(rt.root(), &["first", "second"], 0);
    assert_eq!(rt.git(&dirty, &["rev-parse", "HEAD"]), new);
    assert!(rt.git(&dirty, &["status", "--porcelain"]).is_empty());
    assert!(rt
        .git(rt.root(), &["ls-remote", &url, "refs/heads/trunk"])
        .starts_with(&new));
}
