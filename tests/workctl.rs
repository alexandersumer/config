//! Public workctl safety and migration contracts against real disposable Git repositories.
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};
const BIN: &str = env!("CARGO_BIN_EXE_workctl");
fn env(cmd: &mut Command) -> &mut Command {
    cmd.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .env("NO_COLOR", "1")
        .env("TERM", "dumb");
    for name in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_CONFIG_COUNT",
    ] {
        cmd.env_remove(name);
    }
    cmd
}
fn git(path: &Path, args: &[&str]) -> String {
    let out = env(Command::new("git").current_dir(path))
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}
struct Fixture {
    temp: tempfile::TempDir,
    repo: PathBuf,
    remote: PathBuf,
    worktree: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        Self::with_object_format("sha1")
    }
    fn with_object_format(format: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let repo = root.join("main");
        let remote = root.join("remote.git");
        let worktree = root.join("linked with spaces");
        fs::create_dir(&repo).unwrap();
        git(
            &repo,
            &["init", &format!("--object-format={format}"), "-b", "main"],
        );
        fs::write(repo.join("file"), "published\n").unwrap();
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-m", "initial"]);
        git(
            root,
            &[
                "clone",
                "--bare",
                repo.to_str().unwrap(),
                remote.to_str().unwrap(),
            ],
        );
        git(
            &repo,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "feature",
                worktree.to_str().unwrap(),
            ],
        );
        Self {
            temp,
            repo,
            remote,
            worktree,
        }
    }
    // Existing remote-publication and explicit-data-approval contracts remain
    // meaningful under --strict; pragmatic defaults have separate real-Git tests.
    fn command(&self, extra: &[&str]) -> Command {
        let mut c = Command::new(BIN);
        env(&mut c);
        c.current_dir(&self.repo)
            .args(["git", "worktree", "clean", "--json", "--strict"])
            .args(extra);
        c
    }
    fn run(&self, extra: &[&str]) -> Output {
        self.command(extra).arg(&self.worktree).output().unwrap()
    }
    fn result(&self, extra: &[&str], code: i32) -> serde_json::Value {
        let out = self.run(extra);
        assert_eq!(out.status.code(), Some(code), "{}", text(&out));
        let result = serde_json::from_slice(&out.stdout).expect("stdout is one JSON document");
        assert!(!out.stdout.contains(&0x1b));
        result
    }
    fn unchanged(&self) {
        assert!(self.worktree.join("file").is_file());
        assert!(git(&self.repo, &["worktree", "list", "--porcelain"])
            .contains(self.worktree.to_str().unwrap()));
    }
    fn shim(&self, script: &str) -> PathBuf {
        let real = Command::new("sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap();
        let bin = self.temp.path().join("shim");
        fs::create_dir(&bin).unwrap();
        fs::write(
            bin.join("git"),
            format!(
                "#!/bin/sh\n{script}\nexec '{}' \"$@\"\n",
                String::from_utf8_lossy(&real.stdout).trim()
            ),
        )
        .unwrap();
        fs::set_permissions(bin.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
        bin
    }
}

#[test]
fn relative_remote_publication_preserves_source_resolution_without_mutating_refs() {
    let f = Fixture::new();
    git(&f.repo, &["remote", "set-url", "origin", "../remote.git"]);
    assert!(!git(&f.repo, &["ls-remote", "origin"]).is_empty());
    let refs = git(&f.repo, &["for-each-ref"]);
    let plan = f.result(&[], 0);
    assert_eq!(plan["summary"]["would_remove"], 1);
    f.unchanged();
    assert_eq!(git(&f.repo, &["for-each-ref"]), refs);
    assert!(!f.repo.join(".git/FETCH_HEAD").exists());
    let applied = f.result(&["--apply"], 0);
    assert_eq!(applied["summary"]["removed"], 1);
    assert!(!f.worktree.exists());
    assert_eq!(git(&f.repo, &["for-each-ref"]), refs);
    assert!(!f.repo.join(".git/FETCH_HEAD").exists());
    assert_eq!(
        git(&f.repo, &["worktree", "list", "--porcelain"])
            .lines()
            .filter(|line| line.starts_with("worktree "))
            .count(),
        1
    );
}

#[test]
fn special_coordination_files_refuse_promptly_in_reset_and_cleanup() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    for cleanup in [false, true] {
        let f = Fixture::new();
        let lock = f.repo.join(".git/repo-batch.lock");
        let name = CString::new(lock.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        let mut cmd = Command::new(BIN);
        env(&mut cmd);
        cmd.args(if cleanup {
            vec!["git", "worktree", "clean", "--json", "--timeout", "1"]
        } else {
            vec![
                "git",
                "reset",
                "--json",
                "--timeout",
                "1",
                "--attempts",
                "1",
            ]
        });
        cmd.arg(if cleanup { &f.worktree } else { &f.repo })
            .env("TMPDIR", f.temp.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                let out = child.wait_with_output().unwrap();
                panic!(
                    "special coordination file bypassed deadline: {}",
                    text(&out)
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let out = child.wait_with_output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(r["schema_version"], if cleanup { 2 } else { 1 });
        assert_eq!(r["status"], if cleanup { "blocked" } else { "failed" });
        assert_eq!(
            r["results"][0]["attempts"],
            if cleanup {
                serde_json::Value::Null
            } else {
                serde_json::json!(0)
            }
        );
        let detail = if cleanup {
            &r["results"][0]["reason"]
        } else {
            &r["results"][0]["detail"]
        };
        assert!(detail.as_str().unwrap().contains("repo-batch.lock"), "{r}");
        f.unchanged();
        assert_eq!(git(&f.repo, &["log", "-1", "--format=%s"]), "initial");
    }
}

#[test]
fn cleanup_uses_source_object_format_even_when_init_default_differs() {
    for (source, default) in [("sha256", "sha1"), ("sha1", "sha256")] {
        let f = Fixture::with_object_format(source);
        let out = f
            .command(&[])
            .arg(&f.worktree)
            .env("GIT_DEFAULT_HASH", default)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
        let plan: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(plan["summary"]["would_remove"], 1);
        f.unchanged();
        let out = f
            .command(&["--apply"])
            .arg(&f.worktree)
            .env("GIT_DEFAULT_HASH", default)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
        assert!(!f.worktree.exists());
        assert_eq!(
            git(&f.repo, &["worktree", "list", "--porcelain"])
                .lines()
                .filter(|line| line.starts_with("worktree "))
                .count(),
            1
        );
    }
}

#[test]
fn git_diagnostics_redact_query_values_in_results_and_retained_logs() {
    let f = Fixture::new();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    git(&f.repo, &["remote", "set-url", "origin", &format!("http://127.0.0.1:{port}/repo.git?access_token=query-secret&signature=signed-secret")]);
    let runtime = f.temp.path().join("runtime");
    fs::create_dir(&runtime).unwrap();
    for args in [
        vec!["git", "reset", "--attempts", "1", "--json", "--verbose"],
        vec![
            "git",
            "worktree",
            "clean",
            "--strict",
            "--apply",
            "--json",
            "--verbose",
        ],
    ] {
        let out = env(&mut Command::new(BIN))
            .args(args)
            .arg(&f.repo)
            .env("TMPDIR", &runtime)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(text(&out).contains("[redacted]"), "{}", text(&out));
        assert!(
            !text(&out).contains("query-secret") && !text(&out).contains("signed-secret"),
            "{}",
            text(&out)
        );
        for entry in walkdir::WalkDir::new(&runtime) {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                let bytes = fs::read(entry.path()).unwrap();
                let content = String::from_utf8_lossy(&bytes);
                assert!(
                    !content.contains("query-secret") && !content.contains("signed-secret"),
                    "{}: {content}",
                    entry.path().display()
                );
            }
        }
        f.unchanged();
    }
}

#[test]
fn reset_startup_output_failure_removes_empty_diagnostics() {
    let f = Fixture::new();
    let runtime = f.temp.path().join("runtime");
    fs::create_dir(&runtime).unwrap();
    // Close the reader before launch so the discovery announcement deterministically fails.
    let (reader, writer) = std::os::unix::net::UnixStream::pair().unwrap();
    drop(reader);
    let out = env(&mut Command::new(BIN))
        .args(["git", "reset"])
        .arg(&f.repo)
        .env("TMPDIR", &runtime)
        .stderr(Stdio::from(std::os::fd::OwnedFd::from(writer)))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(
        out.stdout.is_empty(),
        "startup must stop before discovery or reset: {}",
        text(&out)
    );
    assert_eq!(fs::read_dir(runtime).unwrap().count(), 0);
    f.unchanged();
}
fn with_shim(cmd: &mut Command, bin: &Path) {
    cmd.env(
        "PATH",
        format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
    );
}
fn wait(path: &Path) {
    let until = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn preview_is_nonmutating_and_apply_verifies_files_and_registrations() {
    let f = Fixture::new();
    let before = git(&f.repo, &["for-each-ref"]);
    let r = f.result(&[], 0);
    assert_eq!(r["results"][0]["status"], "would remove");
    f.unchanged();
    assert_eq!(before, git(&f.repo, &["for-each-ref"]));
    assert!(!f.repo.join(".git/FETCH_HEAD").exists());
    let r = f.result(&["--apply"], 0);
    assert_eq!(r["results"][0]["status"], "removed");
    assert!(!f.worktree.exists());
    assert!(
        !git(&f.repo, &["worktree", "list", "--porcelain"]).contains(f.worktree.to_str().unwrap())
    );
    assert!(f.repo.join("file").exists());
    assert_eq!(before, git(&f.repo, &["for-each-ref"]));
}
#[test]
fn local_data_needs_a_separate_exact_discard_decision() {
    for kind in ["tracked", "untracked", "ignored", "masked"] {
        let f = Fixture::new();
        match kind {
            "tracked" => {
                fs::write(f.worktree.join("file"), "precious").unwrap();
            }
            "untracked" => {
                fs::write(f.worktree.join("notes"), "precious").unwrap();
            }
            "ignored" => {
                fs::write(f.repo.join(".git/info/exclude"), "notes\n").unwrap();
                fs::write(f.worktree.join("notes"), "precious").unwrap();
            }
            _ => {
                git(&f.worktree, &["update-index", "--assume-unchanged", "file"]);
                fs::write(f.worktree.join("file"), "precious").unwrap();
            }
        }
        let r = f.result(&["--apply"], 1);
        assert_eq!(r["results"][0]["status"], "blocked");
        assert!(r["results"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("--discard-local"));
        f.unchanged();
        let r = f.result(
            &["--apply", "--discard-local", f.worktree.to_str().unwrap()],
            0,
        );
        assert_eq!(r["results"][0]["status"], "removed");
        assert!(!f.worktree.exists());
    }
}

#[test]
fn remote_evidence_rejects_unpublished_and_deleted_remote_branches() {
    let f = Fixture::new();
    fs::write(f.worktree.join("file"), "local commit").unwrap();
    git(&f.worktree, &["commit", "-am", "local"]);
    let r = f.result(&["--apply"], 1);
    assert!(r["results"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("unpublished"));
    f.unchanged();
    git(&f.worktree, &["push", "origin", "feature"]);
    assert_eq!(f.result(&[], 0)["results"][0]["status"], "would remove");
    git(&f.remote, &["update-ref", "-d", "refs/heads/feature"]);
    assert!(!git(&f.repo, &["rev-parse", "refs/remotes/origin/feature"]).is_empty());
    assert!(f.result(&["--apply"], 1)["results"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("unpublished"));
    f.unchanged();
}
#[test]
fn detached_head_is_checked_against_current_publication() {
    let f = Fixture::new();
    git(&f.worktree, &["checkout", "--detach"]);
    fs::write(f.worktree.join("file"), "local").unwrap();
    git(&f.worktree, &["commit", "-am", "detached"]);
    assert!(f.result(&["--apply"], 1)["results"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("unpublished"));
    f.unchanged();
}
#[test]
fn locks_nested_repositories_and_unverifiable_ownership_remain_blockers() {
    for kind in [
        "locked",
        "nested",
        "index-lock",
        "operation",
        "ownership",
        "missing",
    ] {
        let f = Fixture::new();
        let own = PathBuf::from(git(&f.worktree, &["rev-parse", "--absolute-git-dir"]));
        match kind {
            "locked" => {
                git(&f.repo, &["worktree", "lock", f.worktree.to_str().unwrap()]);
            }
            "nested" => {
                let p = f.worktree.join("nested");
                fs::create_dir(&p).unwrap();
                git(&p, &["init"]);
            }
            "index-lock" => {
                fs::write(own.join("index.lock"), "locked").unwrap();
            }
            "operation" => {
                fs::write(own.join("MERGE_HEAD"), git(&f.repo, &["rev-parse", "HEAD"])).unwrap();
            }
            "missing" => {
                fs::remove_dir_all(&f.worktree).unwrap();
            }
            _ => {
                fs::write(own.join("gitdir"), "/unowned/.git\n").unwrap();
            }
        }
        let out = f.command(&["--apply"]).arg(&f.repo).output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{kind}: {}", text(&out));
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(r["results"][0]["status"], "blocked", "{kind}: {r}");
        assert!(f.repo.join("file").exists());
        if kind != "missing" {
            assert!(f.worktree.exists());
        }
    }
}
#[test]
fn scope_overlap_primary_protection_and_partial_failure_are_precise() {
    let f = Fixture::new();
    let other = f.temp.path().join("unpublished");
    git(
        &f.repo,
        &[
            "worktree",
            "add",
            "-b",
            "unpublished",
            other.to_str().unwrap(),
        ],
    );
    fs::write(other.join("file"), "local").unwrap();
    git(&other, &["commit", "-am", "unpublished"]);
    let outside = tempfile::tempdir().unwrap();
    let outside_path = outside.path().join("outside");
    git(
        &f.repo,
        &[
            "worktree",
            "add",
            "-b",
            "outside",
            outside_path.to_str().unwrap(),
        ],
    );
    let out = f
        .command(&["--apply"])
        .arg(f.temp.path())
        .arg(f.temp.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["summary"]["removed"], 1);
    assert_eq!(r["summary"]["blocked"], 1);
    assert_eq!(r["results"].as_array().unwrap().len(), 2);
    assert!(outside_path.exists());
    assert!(other.exists());
    assert!(f.repo.exists());
}
#[test]
fn late_invalid_scope_and_discard_targets_prevent_all_removals() {
    let f = Fixture::new();
    let out = f
        .command(&["--apply"])
        .arg(&f.worktree)
        .arg(f.temp.path().join("missing"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    f.unchanged();
    f.result(&["--apply", "--discard-local", f.repo.to_str().unwrap()], 1);
    f.result(
        &["--apply", "--discard-ignored", f.repo.to_str().unwrap()],
        1,
    );
    f.unchanged();
}
#[test]
fn revalidation_blocks_remote_changes_before_removal() {
    let f = Fixture::new();
    fs::write(f.worktree.join("file"), "commit").unwrap();
    git(&f.worktree, &["commit", "-am", "published"]);
    git(&f.worktree, &["push", "origin", "feature"]);
    let counter = f.temp.path().join("count");
    let bin=f.shim(&format!("for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then if [ -e '{}' ]; then '{}' --git-dir='{}' update-ref -d refs/heads/feature; else touch '{}'; fi; fi; done",counter.display(),git_program(),f.remote.display(),counter.display()));
    let mut cmd = f.command(&["--apply"]);
    cmd.arg(&f.worktree);
    with_shim(&mut cmd, &bin);
    let out = cmd.output().unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    f.unchanged();
}
fn git_program() -> String {
    String::from_utf8(
        Command::new("sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .into()
}
#[test]
fn removal_success_requires_filesystem_and_registration_verification() {
    for state in ["both", "path", "registration"] {
        let f = Fixture::new();
        let head = git(&f.worktree, &["rev-parse", "HEAD"]);
        let bin = f.shim(
            r#"case "$*" in
  *'worktree remove'*)
    echo attempted >> "$PROBE_ATTEMPTS"
    case "$PROBE_STATE" in
      path) "$REAL_GIT" "$@" || exit $?; mkdir "$PROBE_PATH";;
      registration) rm -rf "$PROBE_PATH";;
    esac
    exit 0;;
esac"#,
        );
        let attempts = f.temp.path().join("attempts");
        let mut cmd = f.command(&["--apply"]);
        cmd.arg(&f.worktree)
            .env("PROBE_STATE", state)
            .env("PROBE_PATH", &f.worktree)
            .env("PROBE_ATTEMPTS", &attempts)
            .env("REAL_GIT", git_program());
        with_shim(&mut cmd, &bin);
        let out = cmd.output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{state}: {}", text(&out));
        let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(r["status"], "failed", "{state}: {r}");
        assert_eq!(r["results"][0]["status"], "failed");
        assert_eq!(r["summary"]["removed"], 0);
        assert_eq!(r["summary"]["blocked"], 0);
        assert_eq!(r["summary"]["failed"], 1);
        assert_eq!(r["summary"]["unverified"], 0);
        assert_eq!(r["summary"]["errors"], 1);
        assert_eq!(r["results"][0]["evidence"]["head"], head);
        assert!(r["results"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("did not verify"));
        assert_eq!(f.worktree.exists(), state != "registration");
        assert_eq!(
            git(&f.repo, &["worktree", "list", "--porcelain"])
                .contains(f.worktree.to_str().unwrap()),
            state != "path"
        );
        assert_eq!(fs::read_to_string(attempts).unwrap(), "attempted\n");
        assert_eq!(git(&f.repo, &["status", "--porcelain"]), "");
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), head);
    }
}
#[test]
fn deadline_and_cancellation_terminate_groups_without_retrying_cleanup() {
    for cancel in [false, true] {
        let f = Fixture::new();
        let started = f.temp.path().join("started");
        let escaped = f.temp.path().join("escaped");
        let release = f.temp.path().join("release-descendant");
        let count = f.temp.path().join("count");
        // Release the sentinel only after shutdown; a slow concurrent check must not
        // make it fire while the subprocess is legitimately still running.
        let bin=f.shim(&format!("for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then echo attempt >> '{}'; (touch '{}'; while [ ! -e '{}' ]; do sleep 0.05; done; touch '{}') & sleep 30; fi; done",count.display(),started.display(),release.display(),escaped.display()));
        let mut cmd = f.command(&["--apply", "--timeout", if cancel { "30" } else { "4" }]);
        cmd.arg(&f.worktree);
        with_shim(&mut cmd, &bin);
        let execution = Instant::now();
        let child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        wait(&started);
        let mut concurrent = Command::new(BIN);
        env(&mut concurrent);
        let other = concurrent
            .args(["git", "reset", "--attempts", "1"])
            .arg(&f.repo)
            .output()
            .unwrap();
        assert_eq!(other.status.code(), Some(1));
        assert!(text(&other).contains("Repository is busy"));
        if cancel {
            unsafe {
                libc::kill(child.id() as i32, libc::SIGINT);
            }
        }
        let out = child.wait_with_output().unwrap();
        assert!(
            execution.elapsed() < Duration::from_secs(8),
            "shutdown exceeded its bounded budget: {}",
            text(&out)
        );
        assert_eq!(
            out.status.code(),
            Some(if cancel { 130 } else { 1 }),
            "{}",
            text(&out)
        );
        assert_eq!(fs::read_to_string(count).unwrap(), "attempt\n");
        assert!(!escaped.exists(), "sentinel must wait for explicit release");
        fs::write(release, "release\n").unwrap();
        std::thread::sleep(Duration::from_secs(3));
        assert!(!escaped.exists());
        f.unchanged();
    }
}
#[test]
fn generated_help_completions_and_doctor_share_the_public_contract() {
    for args in [
        vec!["--help"],
        vec!["git", "--help"],
        vec!["git", "reset", "--help"],
        vec!["git", "worktree", "--help"],
        vec!["git", "worktree", "clean", "--help"],
        vec!["doctor", "--help"],
        vec!["completions", "--help"],
    ] {
        let out = Command::new(BIN).args(args).output().unwrap();
        assert!(out.status.success());
        assert!(text(&out).contains("Example"));
        assert!(text(&out).contains("--version"));
    }
    let out = Command::new(BIN)
        .args(["doctor", "--jobs", "4"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let out = Command::new(BIN)
        .args(["completions", "zsh"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(text(&out).contains("worktree"));
    assert!(text(&out).contains("discard-local"));
    let out = env(&mut Command::new(BIN))
        .args(["doctor", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["schema_version"], 1);
    let out = Command::new(env!("CARGO_BIN_EXE_config-tools"))
        .args(["reset-to-origin", "--help"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
}
#[test]
fn atomic_install_migrates_owned_files_preserves_foreign_files_and_works_in_fresh_shells() {
    let home = tempfile::tempdir().unwrap();
    let bin = home.path().join(".local/bin");
    fs::create_dir_all(&bin).unwrap();
    for n in ["config-tools", "reset_to_origin"] {
        fs::copy(env!("CARGO_BIN_EXE_config-tools"), bin.join(n)).unwrap();
    }
    let install = || {
        use std::os::unix::process::CommandExt;
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_config-tools"));
        cmd.args(["install-workctl", "--home"]).arg(home.path());
        unsafe {
            cmd.pre_exec(|| {
                libc::umask(0o002);
                Ok(())
            });
        }
        cmd.output().unwrap()
    };
    let out = install();
    assert!(out.status.success(), "{}", text(&out));
    let expected = fs::read(env!("CARGO_BIN_EXE_config-tools")).unwrap();
    for name in ["workctl", "config-tools"] {
        assert_eq!(fs::read(bin.join(name)).unwrap(), expected);
    }
    assert!(!bin.join("reset_to_origin").exists());
    assert!(home
        .path()
        .join(".local/share/zsh/site-functions/_workctl")
        .exists());
    for name in [
        ".local/share",
        ".local/share/zsh",
        ".local/share/zsh/site-functions",
    ] {
        assert_eq!(
            fs::metadata(home.path().join(name))
                .unwrap()
                .permissions()
                .mode()
                & 0o022,
            0,
            "{name} must not be group or world writable"
        );
    }
    for shell in ["sh", "zsh"] {
        let out = Command::new(shell)
            .args([
                "-c",
                "command workctl --version; command workctl git worktree clean --help",
            ])
            .env("HOME", home.path())
            .env(
                "PATH",
                format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
            )
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
        assert!(text(&out).contains("workctl"));
    }
    symlink(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("zsh"),
        home.path().join(".zsh"),
    )
    .unwrap();
    let config =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("zsh/zshrc")).unwrap();
    let helpers = config
        .split_once("# Git helpers\n")
        .unwrap()
        .1
        .split_once("# Claude Code (auto-update)")
        .unwrap()
        .0;
    let shell_config = home.path().join("workctl-shell-config.zsh");
    fs::write(&shell_config, helpers).unwrap();
    let out = Command::new("zsh")
        .args(["-f", "-c", "reset_to_origin() { return 99; }; alias reset_to_origin='return 98'; source \"$1\" || exit 1; fpath=(\"$HOME/.local/share/zsh/site-functions\" $fpath); autoload -Uz compinit compaudit; compaudit || exit 1; compinit -D; rehash; [[ ${_comps[workctl]} == _workctl ]] && autoload +X _workctl && ! whence reset_to_origin && workctl doctor --json", "verification"])
        .arg(&shell_config)
        .env("HOME", home.path())
        .env("PATH", format!("{}:{}", bin.display(), "/usr/bin:/bin"))
        .output().unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let out = install();
    assert!(out.status.success(), "{}", text(&out));
    fs::write(bin.join("reset_to_origin"), "#!/bin/sh\necho foreign\n").unwrap();
    fs::set_permissions(
        bin.join("reset_to_origin"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let out = install();
    assert!(!out.status.success());
    assert!(text(&out).contains("cannot prove legacy ownership"));
    assert!(fs::read_to_string(bin.join("reset_to_origin"))
        .unwrap()
        .contains("foreign"));
    fs::remove_file(bin.join("reset_to_origin")).unwrap();
    fs::remove_file(bin.join("workctl")).unwrap();
    symlink("foreign", bin.join("workctl")).unwrap();
    assert!(!install().status.success());
    assert_eq!(
        fs::read_link(bin.join("workctl")).unwrap(),
        PathBuf::from("foreign")
    );
}
#[test]
fn reset_json_is_one_document_and_success_leaves_no_diagnostics() {
    let f = Fixture::new();
    let runtime = f.temp.path().join("runtime");
    fs::create_dir(&runtime).unwrap();
    let out = env(&mut Command::new(BIN))
        .args(["git", "reset", "--json", "--attempts", "1"])
        .arg(&f.repo)
        .env("TMPDIR", &runtime)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["schema_version"], 1);
    assert_eq!(r["summary"]["succeeded"], 1);
    assert!(r["diagnostics"].is_null());
    assert_eq!(fs::read_dir(runtime).unwrap().count(), 0);
}

fn terminal(mut command: Command, width: u16) -> (i32, Vec<u8>) {
    use std::io::Read;
    use std::os::fd::FromRawFd;
    let mut master = -1;
    let mut slave = -1;
    let mut size = libc::winsize {
        ws_row: 24,
        ws_col: width,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    assert_eq!(
        unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut size,
            )
        },
        0
    );
    let mut master = unsafe { fs::File::from_raw_fd(master) };
    let slave = unsafe { fs::File::from_raw_fd(slave) };
    command
        .stdin(Stdio::null())
        .stdout(slave.try_clone().unwrap())
        .stderr(slave.try_clone().unwrap());
    let mut child = command.spawn().unwrap();
    drop(command);
    drop(slave);
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            match master.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => bytes.extend_from_slice(&buffer[..n]),
                Err(e) if e.raw_os_error() == Some(libc::EIO) => break,
                Err(e) => panic!("PTY read: {e}"),
            }
        }
        bytes
    });
    let code = child.wait().unwrap().code().unwrap();
    (code, reader.join().unwrap())
}
#[test]
fn terminal_and_redirected_rendering_preserve_paths_and_color_disabled_status_words() {
    let f = Fixture::new();
    for width in [28, 80] {
        let mut cmd = Command::new(BIN);
        env(&mut cmd);
        cmd.env("TERM", "xterm")
            .args(["git", "worktree", "clean"])
            .arg(&f.worktree);
        let (code, bytes) = terminal(cmd, width);
        assert_eq!(code, 0);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains(f.worktree.to_str().unwrap()), "{text}");
        assert!(text.contains("would remove"), "{text}");
        assert!(
            !text.contains("\x1b[32m"),
            "NO_COLOR must suppress status color"
        );
        f.unchanged();
    }
    let mut cmd = Command::new(BIN);
    env(&mut cmd);
    cmd.env_remove("NO_COLOR")
        .env_remove("CLICOLOR")
        .env("TERM", "xterm")
        .args(["git", "worktree", "clean"])
        .arg(&f.worktree);
    let (code, bytes) = terminal(cmd, 80);
    assert_eq!(code, 0);
    assert!(bytes.windows(5).any(|b| b == b"\x1b[32m"));
    let out = env(&mut Command::new(BIN))
        .args(["git", "worktree", "clean"])
        .arg(&f.worktree)
        .env("COLUMNS", "28")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains(f.worktree.to_str().unwrap()));
    assert!(!out.stdout.contains(&0x1b));
    assert!(!out.stderr.contains(&0x1b));
}
#[test]
fn cleanup_closed_output_and_initialized_submodules_never_authorize_removal() {
    let f = Fixture::new();
    let mut child = f
        .command(&[])
        .arg(&f.worktree)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    f.unchanged();
    let oid = git(&f.repo, &["rev-parse", "HEAD"]);
    git(
        &f.worktree,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{oid},submodule"),
        ],
    );
    git(&f.worktree, &["commit", "-m", "submodule"]);
    git(&f.worktree, &["push", "origin", "feature"]);
    fs::create_dir(f.worktree.join("submodule")).unwrap();
    git(&f.worktree.join("submodule"), &["init"]);
    let r = f.result(
        &["--apply", "--discard-local", f.worktree.to_str().unwrap()],
        1,
    );
    assert!(r["results"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("unsupported nested repository"));
    assert!(f.worktree.join("submodule/.git").is_dir());
    assert_eq!(r["summary"]["removed"], 0);
    f.unchanged();
}
#[test]
fn json_preserves_filesystem_path_bytes_without_terminal_decoration() {
    use std::os::unix::ffi::OsStringExt;
    let f = Fixture::new();
    let path = f
        .temp
        .path()
        .join(std::ffi::OsString::from_vec(if cfg!(target_os = "linux") {
            b"linked-\xff".to_vec()
        } else {
            "linked-\n-é".as_bytes().to_vec()
        }));
    let out = env(Command::new("git").current_dir(&f.repo))
        .args(["worktree", "move"])
        .arg(&f.worktree)
        .arg(&path)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let out = f.command(&[]).arg(&path).output().unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let bytes = r["results"][0]["path_bytes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_u64().unwrap() as u8)
        .collect::<Vec<_>>();
    assert_eq!(
        bytes,
        path.canonicalize().unwrap().as_os_str().as_encoded_bytes()
    );
}

#[test]
fn failed_publication_redacts_url_credentials_in_human_and_json_diagnostics() {
    let f = Fixture::new();
    git(
        &f.repo,
        &[
            "remote",
            "set-url",
            "origin",
            "unsupported://private-user:workctl-secret@host/repo",
        ],
    );
    for args in [vec!["--apply"], vec!["--apply", "--json", "--verbose"]] {
        let out = f.command(&args).arg(&f.worktree).output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(text(&out).contains("[redacted]@host"), "{}", text(&out));
        assert!(!text(&out).contains("workctl-secret"));
        assert!(!text(&out).contains("private-user"));
        f.unchanged();
    }
}

#[test]
fn cleanup_reports_post_removal_errors_from_verified_state() {
    let real_git = Command::new("sh")
        .args(["-c", "command -v git"])
        .output()
        .unwrap();
    let real_git = String::from_utf8(real_git.stdout).unwrap();
    for (mode, expected, removed) in [
        ("after", "removed", true),
        ("before", "failed", false),
        ("verify", "unverified", true),
    ] {
        for json in [true, false] {
            let f = Fixture::new();
            let bin = f.temp.path().join("bin");
            fs::create_dir(&bin).unwrap();
            let wrapper = bin.join("git");
            fs::write(&wrapper, r#"#!/bin/sh
case "$*" in
  *"worktree remove"*)
    echo attempted >> "$PROBE_ATTEMPTS"
    if [ "$PROBE_MODE" = before ]; then echo injected refusal >&2; exit 1; fi
    "$REAL_GIT" "$@" || exit $?
    touch "$PROBE_STAMP"
    if [ "$PROBE_MODE" = after ]; then echo injected reporting failure >&2; exit 1; fi
    exit 0;;
  *"worktree list"*)
    if [ "$PROBE_MODE" = verify ] && [ -f "$PROBE_STAMP" ]; then echo injected verification failure >&2; exit 1; fi;;
esac
exec "$REAL_GIT" "$@"
"#).unwrap();
            fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
            let attempts = f.temp.path().join("attempts");
            let canonical_path = f.worktree.canonicalize().unwrap();
            let mut command = Command::new(BIN);
            env(&mut command).args(["git", "worktree", "clean", "--apply"]);
            if json {
                command.arg("--json");
            }
            let out = command
                .arg(&f.worktree)
                .env(
                    "PATH",
                    format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
                )
                .env("REAL_GIT", real_git.trim())
                .env("PROBE_MODE", mode)
                .env("PROBE_STAMP", f.temp.path().join("stamp"))
                .env("PROBE_ATTEMPTS", &attempts)
                .output()
                .unwrap();
            assert_eq!(out.status.code(), Some(1), "{}", text(&out));
            if json {
                let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
                assert_eq!(r["results"][0]["status"], expected, "{mode}: {r}");
                assert_eq!(r["schema_version"], 2);
                assert_eq!(r["status"], "failed");
                assert_eq!(r["summary"]["blocked"], 0);
                assert_eq!(r["summary"]["errors"], 1);
                assert_eq!(r["summary"]["removed"], usize::from(expected == "removed"));
                assert_eq!(r["summary"]["failed"], usize::from(expected == "failed"));
                assert_eq!(
                    r["summary"]["unverified"],
                    usize::from(expected == "unverified")
                );
                assert!(r["results"][0]["evidence"]["head"].is_string());
                assert!(r["results"][0]["reason"].is_string());
                assert!(!out.stdout.contains(&0x1b));
            } else {
                let output = String::from_utf8(out.stdout).unwrap();
                assert!(
                    output.contains(&format!("{expected}: {}", canonical_path.display())),
                    "{output}"
                );
                assert!(output.contains("Completed with errors"));
                assert!(output.contains("0 blocked"));
            }
            assert_eq!(f.worktree.exists(), !removed);
            let registrations = git(&f.repo, &["worktree", "list", "--porcelain"]);
            assert_eq!(
                registrations.contains(f.worktree.to_str().unwrap()),
                !removed
            );
            assert_eq!(fs::read_to_string(attempts).unwrap(), "attempted\n");
            assert!(f.repo.exists());
        }
    }
}

#[test]
fn revalidation_preserves_local_data_created_during_final_publication() {
    let f = Fixture::new();
    let head = git(&f.worktree, &["rev-parse", "HEAD"]);
    let notes = f.worktree.join("late-notes");
    let fetches = f.temp.path().join("fetches");
    let attempted = f.temp.path().join("removal-attempted");
    let bin = f.shim(
        r#"for arg in "$@"; do
  if [ "$arg" = fetch ]; then
    if [ -f "$PROBE_FETCHES" ]; then printf 'precious\n' > "$PROBE_NOTES"; fi
    echo fetch >> "$PROBE_FETCHES"
  fi
  if [ "$arg" = remove ]; then touch "$PROBE_ATTEMPTED"; fi
done"#,
    );
    let mut cmd = f.command(&["--apply", "--discard-local"]);
    cmd.arg(&f.worktree)
        .arg(&f.worktree)
        .env("PROBE_FETCHES", &fetches)
        .env("PROBE_NOTES", &notes)
        .env("PROBE_ATTEMPTED", &attempted);
    with_shim(&mut cmd, &bin);
    let out = cmd.output().unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["status"], "blocked");
    assert_eq!(r["results"][0]["status"], "blocked");
    assert_eq!(r["summary"]["blocked"], 1);
    assert_eq!(r["summary"]["removed"], 0);
    assert_eq!(r["summary"]["errors"], 0);
    assert!(r["results"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("changed"));
    assert_eq!(fs::read_to_string(fetches).unwrap(), "fetch\nfetch\n");
    assert!(
        !attempted.exists(),
        "changed data must block before removal"
    );
    assert_eq!(fs::read_to_string(notes).unwrap(), "precious\n");
    assert_eq!(git(&f.worktree, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&f.repo, &["status", "--porcelain"]), "");
    f.unchanged();
}

#[test]
fn cancellation_after_native_removal_reports_unverified_and_releases_lock() {
    use std::os::unix::process::CommandExt;
    let f = Fixture::new();
    let head = git(&f.worktree, &["rev-parse", "HEAD"]);
    let started = f.temp.path().join("removed");
    let release = f.temp.path().join("release");
    let escaped = f.temp.path().join("escaped");
    let attempts = f.temp.path().join("attempts");
    let bin = f.shim(r#"case "$*" in
  *'worktree remove'*)
    echo attempted >> "$PROBE_ATTEMPTS"
    "$REAL_GIT" "$@" || exit $?
    (touch "$PROBE_STARTED"; while [ ! -f "$PROBE_RELEASE" ]; do sleep 0.05; done; touch "$PROBE_ESCAPED") &
    sleep 30
    exit 0;;
esac"#);
    let mut cmd = f.command(&["--apply", "--timeout", "30"]);
    cmd.arg(&f.worktree)
        .env("REAL_GIT", git_program())
        .env("PROBE_ATTEMPTS", &attempts)
        .env("PROBE_STARTED", &started)
        .env("PROBE_RELEASE", &release)
        .env("PROBE_ESCAPED", &escaped)
        .process_group(0);
    with_shim(&mut cmd, &bin);
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !started.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    let reached_removal = started.exists();
    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    let timed_out = child.try_wait().unwrap().is_none();
    if timed_out {
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        reached_removal && !timed_out,
        "cleanup must stop promptly after native removal: {}",
        text(&out)
    );
    assert_eq!(out.status.code(), Some(130), "{}", text(&out));
    let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["status"], "interrupted");
    assert_eq!(r["results"][0]["status"], "unverified");
    assert_eq!(r["summary"]["removed"], 0);
    assert_eq!(r["summary"]["blocked"], 0);
    assert_eq!(r["summary"]["interrupted"], 0);
    assert_eq!(r["summary"]["unverified"], 1);
    assert_eq!(r["summary"]["errors"], 1);
    assert_eq!(r["results"][0]["evidence"]["head"], head);
    assert!(r["results"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("Interrupted"));
    assert!(!f.worktree.exists());
    assert!(
        !git(&f.repo, &["worktree", "list", "--porcelain"]).contains(f.worktree.to_str().unwrap())
    );
    assert_eq!(fs::read_to_string(attempts).unwrap(), "attempted\n");
    fs::write(release, "release\n").unwrap();
    std::thread::sleep(Duration::from_secs(3));
    assert!(
        !escaped.exists(),
        "removal descendants must not survive cancellation"
    );
    let next = f.command(&[]).arg(&f.repo).output().unwrap();
    assert!(
        next.status.success(),
        "common-directory lock must be released: {}",
        text(&next)
    );
    assert_eq!(git(&f.repo, &["status", "--porcelain"]), "");
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), head);
}

#[test]
fn empty_cleanup_announces_discovery_without_execution_activity() {
    let root = tempfile::tempdir().unwrap();
    for apply in [false, true] {
        for json in [false, true] {
            let mut cmd = Command::new(BIN);
            env(&mut cmd).args(["git", "worktree", "clean"]);
            if apply {
                cmd.arg("--apply");
            }
            if json {
                cmd.arg("--json");
            }
            let out = cmd.arg(root.path()).output().unwrap();
            assert!(out.status.success(), "{}", text(&out));
            assert_eq!(
                String::from_utf8_lossy(&out.stderr),
                "Discovering worktrees…\n",
                "{}",
                text(&out)
            );
            if json {
                let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
                assert_eq!(r["status"], if apply { "completed" } else { "planned" });
                assert_eq!(r["results"], serde_json::json!([]));
                for key in [
                    "removed",
                    "would_remove",
                    "blocked",
                    "failed",
                    "unverified",
                    "errors",
                    "interrupted",
                ] {
                    assert_eq!(r["summary"][key], 0);
                }
            } else {
                let output = String::from_utf8_lossy(&out.stdout);
                assert!(!output.contains("\n\n\n"), "{output}");
                assert!(output.contains(if apply { "Completed" } else { "Removal plan" }));
                assert!(output.contains(if apply { "0 removed" } else { "0 would remove" }));
            }
        }
    }
}

#[test]
fn doctor_startup_failure_still_emits_a_structured_result() {
    let root = tempfile::tempdir().unwrap();
    let deleted = root.path().join("deleted-cwd");
    fs::create_dir(&deleted).unwrap();
    let out = env(&mut Command::new("/bin/sh"))
        .args([
            "-c",
            r#"cd "$1" && rmdir "$1" && exec "$2" doctor --json"#,
            "probe",
        ])
        .arg(&deleted)
        .arg(BIN)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    let r: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("failed doctor must emit one JSON result");
    assert_eq!(r["schema_version"], 1);
    assert_eq!(r["operation"], "doctor");
    assert_eq!(r["status"], "failed");
    assert!(r["error"].as_str().is_some_and(|e| !e.is_empty()));
    assert!(!out.stdout.contains(&0x1b));
}

#[test]
fn doctor_git_version_is_safe_in_human_output() {
    let f = Fixture::new();
    let bin = f.shim(r#"if [ "$1" = --version ]; then printf 'git version 2.54.0\033[31m\rforged\n'; exit 0; fi"#);
    let mut cmd = Command::new(BIN);
    env(&mut cmd).arg("doctor");
    with_shim(&mut cmd, &bin);
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "{}", text(&out));
    assert!(!out.stdout.contains(&0x1b));
    assert!(!out.stdout.contains(&b'\r'));
    assert!(String::from_utf8_lossy(&out.stdout).contains("\\u{1b}"));
}

#[test]
fn discovery_is_visible_before_git_finishes_and_counts_toward_elapsed() {
    for operation in [vec!["git", "reset"], vec!["git", "worktree", "clean"]] {
        let f = Fixture::new();
        let sentinel = f.temp.path().join("discovering");
        let stderr = f.temp.path().join("stderr");
        let bin = f.shim(&format!(
            "if [ ! -f '{}' ]; then touch '{}'; sleep 2; fi",
            sentinel.display(),
            sentinel.display()
        ));
        let mut cmd = Command::new(BIN);
        env(&mut cmd).arg("--json").args(&operation).arg(&f.repo);
        with_shim(&mut cmd, &bin);
        let child = cmd
            .stdout(Stdio::piped())
            .stderr(fs::File::create(&stderr).unwrap())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !sentinel.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let early = fs::read_to_string(&stderr).unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "{}", text(&out));
        assert!(
            early.contains("Discovering"),
            "discovery must be visible before its first Git probe completes: {early:?}"
        );
        let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert!(
            result["summary"]["elapsed_seconds"].as_f64().unwrap() >= 2.0,
            "elapsed must include discovery: {result}"
        );
    }
}

#[test]
fn cancellation_during_discovery_reports_interruption_without_starting_mutation() {
    for operation in [
        vec!["git", "reset"],
        vec!["git", "worktree", "clean", "--apply"],
    ] {
        let f = Fixture::new();
        let sentinel = f.temp.path().join("discovering");
        let bin = f.shim(&format!("touch '{}'; sleep 30", sentinel.display()));
        let mut cmd = Command::new(BIN);
        env(&mut cmd).arg("--json").args(&operation).arg(&f.repo);
        with_shim(&mut cmd, &bin);
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !sentinel.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let reached_probe = sentinel.exists();
        unsafe {
            libc::kill(child.id() as i32, libc::SIGINT);
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let timed_out = child.try_wait().unwrap().is_none();
        if timed_out {
            child.kill().unwrap();
        }
        let out = child.wait_with_output().unwrap();
        assert!(reached_probe && !timed_out, "{}", text(&out));
        assert_eq!(out.status.code(), Some(130), "{}", text(&out));
        let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result["status"], "interrupted");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("Discovering"), "{stderr}");
        assert!(
            !stderr.contains("Resetting checkouts")
                && !stderr.contains("Revalidating and removing"),
            "{stderr}"
        );
        f.unchanged();
        assert_eq!(git(&f.repo, &["status", "--porcelain"]), "");
    }
}

#[test]
fn published_empty_gitlinks_are_eligible_without_discarding_submodule_data() {
    for mapped in [false, true] {
        let f = Fixture::new();
        let oid = git(&f.repo, &["rev-parse", "HEAD"]);
        git(
            &f.worktree,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("160000,{oid},submodule"),
            ],
        );
        fs::create_dir(f.worktree.join("submodule")).unwrap();
        if mapped {
            fs::write(
                f.worktree.join(".gitmodules"),
                "[submodule \"submodule\"]\npath = submodule\nurl = ../module.git\n",
            )
            .unwrap();
            git(&f.worktree, &["add", ".gitmodules"]);
        }
        git(&f.worktree, &["commit", "-m", "empty gitlink"]);
        git(&f.worktree, &["push", "origin", "feature"]);
        let r = f.result(&[], 0);
        assert_eq!(r["summary"]["would_remove"], 1);
        f.unchanged();
        let r = f.result(&["--apply"], 0);
        assert_eq!(r["summary"]["removed"], 1);
        assert!(!f.worktree.exists());
    }
}

#[test]
fn ignored_only_approval_never_discards_other_local_data() {
    for kind in ["ignored", "tracked", "untracked", "masked"] {
        let f = Fixture::new();
        fs::write(f.repo.join(".git/info/exclude"), "cache\n").unwrap();
        fs::write(f.worktree.join("cache"), "ignored data").unwrap();
        match kind {
            "tracked" => {
                fs::write(f.worktree.join("file"), "precious tracked").unwrap();
            }
            "untracked" => {
                fs::write(f.worktree.join("evidence"), "precious untracked").unwrap();
            }
            "masked" => {
                git(&f.worktree, &["update-index", "--assume-unchanged", "file"]);
                fs::write(f.worktree.join("file"), "masked data").unwrap();
            }
            _ => {}
        }
        let blocked = f.result(&[], 1);
        assert!(
            blocked["results"][0]["reason"]
                .as_str()
                .unwrap()
                .contains("cache"),
            "{blocked}"
        );
        let ignored = f.worktree.to_str().unwrap();
        let code = if kind == "ignored" { 0 } else { 1 };
        let preview = f.result(&["--discard-ignored", ignored], code);
        assert_eq!(
            preview["summary"]["would_remove"],
            usize::from(kind == "ignored")
        );
        f.unchanged();
        let applied = f.result(&["--apply", "--discard-ignored", ignored], code);
        if kind == "ignored" {
            assert_eq!(applied["summary"]["removed"], 1);
            assert!(!f.worktree.exists());
        } else {
            assert_eq!(applied["summary"]["blocked"], 1);
            assert_eq!(
                fs::read_to_string(f.worktree.join("cache")).unwrap(),
                "ignored data"
            );
            f.unchanged();
        }
    }
}

#[test]
fn populated_gitlinks_and_private_module_stores_remain_protected() {
    for kind in ["populated", "private objects", "symlink"] {
        let f = Fixture::new();
        let oid = git(&f.repo, &["rev-parse", "HEAD"]);
        git(
            &f.worktree,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("160000,{oid},submodule"),
            ],
        );
        git(&f.worktree, &["commit", "-m", "gitlink"]);
        git(&f.worktree, &["push", "origin", "feature"]);
        let owned = PathBuf::from(git(&f.worktree, &["rev-parse", "--absolute-git-dir"]));
        let protected = match kind {
            "populated" => {
                let dir = f.worktree.join("submodule");
                fs::create_dir(&dir).unwrap();
                dir.join("precious")
            }
            "private objects" => {
                fs::create_dir_all(owned.join("modules/module/objects")).unwrap();
                owned.join("modules/module/objects/precious")
            }
            _ => {
                let dir = f.temp.path().join("outside");
                fs::create_dir(&dir).unwrap();
                symlink(&dir, f.worktree.join("submodule")).unwrap();
                dir.join("precious")
            }
        };
        fs::write(&protected, "precious data").unwrap();
        for approval in ["--discard-local", "--discard-ignored"] {
            let r = f.result(&["--apply", approval, f.worktree.to_str().unwrap()], 1);
            assert_eq!(r["summary"]["blocked"], 1, "{kind}: {r}");
            assert!(
                r["results"][0]["reason"]
                    .as_str()
                    .unwrap()
                    .contains("submodule"),
                "{kind}: {r}"
            );
            assert_eq!(fs::read_to_string(&protected).unwrap(), "precious data");
            f.unchanged();
        }
    }
}

#[test]
fn ignored_only_removal_keeps_native_git_protection_for_last_moment_untracked_files() {
    let f = Fixture::new();
    fs::write(f.repo.join(".git/info/exclude"), "cache\n").unwrap();
    fs::write(f.worktree.join("cache"), "ignored data").unwrap();
    let evidence = f.worktree.join("last-moment-evidence");
    let bin = f.shim(&format!(
        "case \"$*\" in *'worktree remove'*) printf precious > '{}' ;; esac",
        evidence.display()
    ));
    let mut cmd = f.command(&["--apply", "--discard-ignored", f.worktree.to_str().unwrap()]);
    with_shim(&mut cmd, &bin);
    let out = cmd.arg(&f.worktree).output().unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["summary"]["removed"], 0);
    assert_eq!(r["summary"]["failed"], 1);
    assert_eq!(fs::read_to_string(evidence).unwrap(), "precious");
    f.unchanged();
}

#[test]
fn shallow_publication_uses_fresh_evidence_and_preserves_source_boundaries() {
    for unpublished in [false, true] {
        let mut f = Fixture::new();
        for revision in 1..=3 {
            fs::write(
                f.repo.join("file"),
                format!("published revision {revision}\n"),
            )
            .unwrap();
            git(&f.repo, &["add", "file"]);
            git(&f.repo, &["commit", "-m", "published revision"]);
        }
        git(&f.repo, &["push", "origin", "main"]);
        let shallow = f.temp.path().join("shallow primary");
        git(
            f.temp.path(),
            &[
                "clone",
                "--depth",
                "2",
                &format!("file://{}", f.remote.display()),
                shallow.to_str().unwrap(),
            ],
        );
        let linked = f.temp.path().join("shallow linked");
        git(
            &shallow,
            &[
                "worktree",
                "add",
                "-b",
                "shallow-feature",
                linked.to_str().unwrap(),
                "HEAD~1",
            ],
        );
        f.repo = shallow;
        f.worktree = linked;
        assert_eq!(
            git(&f.repo, &["rev-parse", "--is-shallow-repository"]),
            "true"
        );
        if unpublished {
            fs::write(f.worktree.join("file"), "unpublished data\n").unwrap();
            git(&f.worktree, &["add", "file"]);
            git(&f.worktree, &["commit", "-m", "unpublished"]);
        }
        let boundaries = fs::read(f.repo.join(".git/shallow")).unwrap();
        let refs = git(&f.repo, &["for-each-ref"]);
        let head = git(&f.worktree, &["rev-parse", "HEAD"]);
        let code = if unpublished { 1 } else { 0 };
        let plan = f.result(&[], code);
        assert_eq!(plan["summary"]["would_remove"], usize::from(!unpublished));
        f.unchanged();
        let applied = f.result(&["--apply"], code);
        assert_eq!(applied["summary"]["removed"], usize::from(!unpublished));
        assert_eq!(fs::read(f.repo.join(".git/shallow")).unwrap(), boundaries);
        assert_eq!(git(&f.repo, &["for-each-ref"]), refs);
        assert!(!f.repo.join(".git/FETCH_HEAD").exists());
        if unpublished {
            assert_eq!(git(&f.worktree, &["rev-parse", "HEAD"]), head);
            f.unchanged();
        } else {
            assert!(!f.worktree.exists());
        }
    }
}

#[test]
fn publication_handles_case_colliding_remote_refs_without_source_migration() {
    let mut f = Fixture::new();
    let quoted = f.temp.path().join("remote \"quoted\".git");
    fs::rename(&f.remote, &quoted).unwrap();
    f.remote = quoted;
    git(
        &f.repo,
        &["remote", "set-url", "origin", f.remote.to_str().unwrap()],
    );
    let old = git(&f.repo, &["rev-parse", "HEAD"]);
    fs::write(f.repo.join("file"), "new published\n").unwrap();
    git(&f.repo, &["add", "file"]);
    git(&f.repo, &["commit", "-m", "new published"]);
    git(&f.repo, &["push", "origin", "main"]);
    let new = git(&f.repo, &["rev-parse", "HEAD"]);
    // Packed refs can advertise both names on a case-insensitive filesystem.
    fs::write(f.remote.join("packed-refs"), format!("# pack-refs with: peeled fully-peeled sorted\n{old} refs/heads/Feature\n{new} refs/heads/feature\n{new} refs/heads/quoted\"name\n")).unwrap();
    let advertised = git(&f.repo, &["ls-remote", "--heads", "origin"]);
    assert!(advertised.contains("refs/heads/Feature") && advertised.contains("refs/heads/feature"));
    let config = fs::read(f.repo.join(".git/config")).unwrap();
    let refs = git(&f.repo, &["for-each-ref"]);
    let plan = f.result(&[], 0);
    assert_eq!(plan["summary"]["would_remove"], 1);
    f.unchanged();
    let applied = f.result(&["--apply"], 0);
    assert_eq!(applied["summary"]["removed"], 1);
    assert_eq!(fs::read(f.repo.join(".git/config")).unwrap(), config);
    assert_eq!(git(&f.repo, &["for-each-ref"]), refs);
    assert!(!f.repo.join(".git/FETCH_HEAD").exists());
}

#[test]
fn publication_resolves_opposing_url_rewrites_once() {
    let f = Fixture::new();
    let alias = format!("file://{}/unavailable.git", f.temp.path().display());
    let actual = format!("file://{}", f.remote.display());
    git(&f.repo, &["config", "remote.origin.url", &alias]);
    git(
        &f.repo,
        &["config", &format!("url.{actual}.insteadOf"), &alias],
    );
    git(
        &f.repo,
        &["config", &format!("url.{alias}.insteadOf"), &actual],
    );
    assert_eq!(git(&f.repo, &["remote", "get-url", "origin"]), actual);
    let global = f.temp.path().join("global-config");
    git(
        f.temp.path(),
        &[
            "config",
            "--file",
            global.to_str().unwrap(),
            &format!("url.{alias}.insteadOf"),
            &actual,
        ],
    );
    let refs = git(&f.repo, &["for-each-ref"]);
    for apply in [false, true] {
        let mut cmd = f.command(if apply { &["--apply"] } else { &[] });
        cmd.arg(&f.worktree).env("GIT_CONFIG_GLOBAL", &global);
        let out = cmd.output().unwrap();
        assert_eq!(out.status.code(), Some(0), "{}", text(&out));
        let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(
            result["summary"][if apply { "removed" } else { "would_remove" }],
            1
        );
        if !apply {
            f.unchanged();
        }
    }
    assert_eq!(git(&f.repo, &["for-each-ref"]), refs);
}

#[test]
fn publication_fetches_commit_evidence_without_unneeded_file_contents() {
    let f = Fixture::new();
    git(&f.remote, &["config", "uploadpack.allowFilter", "true"]);
    git(
        &f.repo,
        &[
            "remote",
            "set-url",
            "origin",
            &format!("file://{}", f.remote.display()),
        ],
    );
    // This published content is absent from the source alternate object store.
    let publisher = f.temp.path().join("publisher");
    git(
        f.temp.path(),
        &[
            "clone",
            f.remote.to_str().unwrap(),
            publisher.to_str().unwrap(),
        ],
    );
    fs::write(
        publisher.join("remote-only-blob"),
        vec![b'x'; 4 * 1024 * 1024],
    )
    .unwrap();
    git(&publisher, &["add", "."]);
    git(&publisher, &["commit", "-m", "remote content"]);
    git(&publisher, &["push", "origin", "main"]);
    let remote_blob = git(&publisher, &["rev-parse", "HEAD:remote-only-blob"]);
    let remote_tree = git(&publisher, &["rev-parse", "HEAD^{tree}"]);
    let packs = f.temp.path().join("evidence-packs");
    let bin = f.shim(
        r#"
case " $* " in
  *" fetch "*)
    "$REAL_GIT" "$@" || exit $?
    for arg in "$@"; do
      case "$arg" in */workctl-publication-*)
        "$REAL_GIT" --git-dir "$arg" cat-file --batch-all-objects --batch-check >> "$PACK_RECORD"
      ;; esac
    done
    exit 0
  ;;
esac
"#,
    );
    let mut cmd = f.command(&[]);
    cmd.arg(&f.worktree)
        .env("REAL_GIT", git_program())
        .env("PACK_RECORD", &packs);
    with_shim(&mut cmd, &bin);
    let out = cmd.output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let packed = fs::read_to_string(&packs).unwrap();
    assert!(
        packed
            .lines()
            .any(|line| line.split_whitespace().nth(1) == Some("commit")),
        "{packed}"
    );
    assert!(
        !packed.contains(&remote_blob),
        "remote-only blob transferred: {packed}"
    );
    assert!(
        !packed.contains(&remote_tree),
        "remote-only tree transferred: {packed}"
    );
    f.unchanged();
}

#[test]
fn shallow_publication_tries_the_current_default_branch_before_all_remote_history() {
    let mut f = Fixture::new();
    let shallow = f.temp.path().join("shallow");
    git(
        f.temp.path(),
        &[
            "clone",
            "--depth",
            "1",
            &format!("file://{}", f.remote.display()),
            shallow.to_str().unwrap(),
        ],
    );
    let linked = f.temp.path().join("shallow-linked");
    git(
        &shallow,
        &[
            "worktree",
            "add",
            "-b",
            "shallow-feature",
            linked.to_str().unwrap(),
        ],
    );
    f.repo = shallow;
    f.worktree = linked;
    let bin = f.shim(r#"
case " $* " in
  *" fetch "*)
    case " $* " in *" +refs/heads/main:refs/remotes/evidence/"*) ;; *) echo 'unnecessary whole-remote deepening' >&2; exit 91;; esac
  ;;
esac
"#);
    let boundaries = fs::read(f.repo.join(".git/shallow")).unwrap();
    let mut cmd = f.command(&["--apply"]);
    cmd.arg(&f.worktree);
    with_shim(&mut cmd, &bin);
    let out = cmd.output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["summary"]["removed"], 1);
    assert_eq!(fs::read(f.repo.join(".git/shallow")).unwrap(), boundaries);
}

#[test]
fn publication_does_not_launch_unsupervised_automatic_maintenance() {
    let f = Fixture::new();
    let mut cmd = f.command(&["--verbose"]);
    cmd.arg(&f.worktree).env("GIT_TRACE", "1");
    let out = cmd.output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(
        !text(&out).contains("maintenance run --auto"),
        "automatic maintenance escaped the supervised operation: {}",
        text(&out)
    );
    f.unchanged();
}

#[test]
fn publication_tolerates_an_unrelated_remote_branch_disappearing_during_fetch() {
    let f = Fixture::new();
    git(&f.remote, &["branch", "transient"]);
    let bin = f.shim(r#"
case " $* " in *" fetch "*) "$REAL_GIT" --git-dir "$PROBE_REMOTE" update-ref -d refs/heads/transient;; esac
"#);
    let mut cmd = f.command(&["--apply"]);
    cmd.arg(&f.worktree)
        .env("REAL_GIT", git_program())
        .env("PROBE_REMOTE", &f.remote);
    with_shim(&mut cmd, &bin);
    let out = cmd.output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["summary"]["removed"], 1);
    assert!(!f.worktree.exists());
}

#[test]
fn publication_preserves_case_colliding_refs_with_the_older_git_fallback() {
    let f = Fixture::new();
    let head = git(&f.repo, &["rev-parse", "HEAD"]);
    fs::write(f.remote.join("packed-refs"), format!("# pack-refs with: peeled fully-peeled sorted\n{head} refs/heads/Feature\n{head} refs/heads/feature\n")).unwrap();
    let bin = f.shim("if [ \"$1\" = --version ]; then echo 'git version 2.44.0'; exit 0; fi");
    let config = fs::read(f.repo.join(".git/config")).unwrap();
    let refs = git(&f.repo, &["for-each-ref"]);
    for apply in [false, true] {
        let mut cmd = f.command(if apply { &["--apply"] } else { &[] });
        cmd.arg(&f.worktree);
        with_shim(&mut cmd, &bin);
        let out = cmd.output().unwrap();
        assert_eq!(out.status.code(), Some(0), "{}", text(&out));
        let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(
            result["summary"][if apply { "removed" } else { "would_remove" }],
            1
        );
    }
    assert_eq!(fs::read(f.repo.join(".git/config")).unwrap(), config);
    assert_eq!(git(&f.repo, &["for-each-ref"]), refs);
}

#[test]
fn unapproved_local_data_is_reported_before_scanning_protected_ignored_contents() {
    let f = Fixture::new();
    fs::write(f.worktree.join(".gitignore"), "generated/\n").unwrap();
    git(&f.worktree, &["add", ".gitignore"]);
    git(&f.worktree, &["commit", "-m", "ignore generated files"]);
    git(&f.worktree, &["push", "origin", "feature"]);
    let generated = f.worktree.join("generated");
    fs::create_dir(&generated).unwrap();
    fs::write(generated.join("precious-cache"), "preserve").unwrap();
    struct Restore(PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700)).unwrap();
        }
    }
    let _restore = Restore(generated.clone());
    fs::set_permissions(&generated, fs::Permissions::from_mode(0o000)).unwrap();
    let blocked = f.result(&[], 1);
    assert!(
        blocked["results"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("local data"),
        "{blocked}"
    );
    assert!(
        blocked["results"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("generated/"),
        "{blocked}"
    );
    let approved = f.result(
        &["--apply", "--discard-ignored", f.worktree.to_str().unwrap()],
        1,
    );
    assert!(
        approved["results"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("Permission denied"),
        "{approved}"
    );
    f.unchanged();
}

#[test]
fn cleanup_next_steps_are_scoped_executable_and_preserve_protected_data() {
    let mut f = Fixture::new();
    let unusual = f.temp.path().join("linked '; touch escaped; #\n\x1b");
    git(
        &f.repo,
        &[
            "worktree",
            "move",
            f.worktree.to_str().unwrap(),
            unusual.to_str().unwrap(),
        ],
    );
    f.worktree = unusual;
    fs::write(f.worktree.join(".gitignore"), "cache/\n").unwrap();
    git(&f.worktree, &["add", ".gitignore"]);
    git(&f.worktree, &["commit", "-m", "ignore cache"]);
    git(&f.worktree, &["push", "origin", "feature"]);
    fs::create_dir(f.worktree.join("cache")).unwrap();
    fs::write(
        f.worktree.join("cache/data"),
        "preserve unless explicitly approved",
    )
    .unwrap();
    let blocked = f.result(&[], 1);
    let step = &blocked["results"][0]["next_step"];
    assert_eq!(step["action"], "preview_ignored_removal");
    assert_eq!(step["destructive"], false);
    let command = step["command"].as_str().unwrap();
    assert!(!command.contains(['\n', '\x1b']));
    let mut shell = Command::new("zsh");
    env(&mut shell);
    shell
        .current_dir(f.temp.path())
        .args(["-f", "-c", command])
        .env(
            "PATH",
            format!(
                "{}:{}",
                Path::new(BIN).parent().unwrap().display(),
                std::env::var("PATH").unwrap()
            ),
        );
    let out = shell.output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(!f.temp.path().join("escaped").exists());
    f.unchanged();
    assert!(f.worktree.join("cache/data").is_file());
    fs::write(f.worktree.join("untracked-evidence"), "keep").unwrap();
    let blocked = f.result(&[], 1);
    let step = &blocked["results"][0]["next_step"];
    assert_eq!(step["action"], "inspect_local_data");
    assert_eq!(step["destructive"], false);
    assert!(!step["command"]
        .as_str()
        .unwrap()
        .contains("--discard-local"));
    assert!(f.worktree.join("untracked-evidence").is_file());
}

#[test]
fn cleanup_human_guidance_distinguishes_preview_and_current_state_revalidation() {
    let f = Fixture::new();
    let out = f.command(&[]).arg(&f.worktree).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        result["results"][0]["next_step"]["action"],
        "apply_eligible_removal"
    );
    let mut cmd = Command::new(BIN);
    env(&mut cmd);
    let human = cmd
        .args(["git", "worktree", "clean"])
        .arg(&f.worktree)
        .output()
        .unwrap();
    assert_eq!(human.status.code(), Some(0));
    let human = text(&human);
    assert!(
        human.contains("Next: workctl git worktree clean --apply"),
        "{human}"
    );
    assert!(human.contains("Preview only; nothing removed."), "{human}");
    assert!(human.contains("Apply rechecks current state"), "{human}");
    assert!(
        human.contains("tracked changes and untracked files stay protected"),
        "{human}"
    );
    f.unchanged();
}

#[test]
fn exact_commit_preservation_allows_offline_cleanup_without_losing_unpublished_history() {
    for format in ["sha1", "sha256"] {
        let f = Fixture::with_object_format(format);
        fs::write(f.worktree.join("file"), "unpublished committed work").unwrap();
        git(&f.worktree, &["add", "file"]);
        git(&f.worktree, &["commit", "-m", "unpublished"]);
        let head = git(&f.worktree, &["rev-parse", "HEAD"]);
        assert_eq!(f.result(&[], 1)["summary"]["blocked"], 1);
        git(&f.repo, &["remote", "remove", "origin"]);
        let before = git(&f.repo, &["for-each-ref"]);
        let extra = ["--preserve-commits", f.worktree.to_str().unwrap()];
        let plan = f.result(&extra, 0);
        assert_eq!(plan["summary"]["would_remove"], 1);
        assert_eq!(git(&f.repo, &["for-each-ref"]), before);
        f.unchanged();
        let applied = f.result(
            &[
                "--apply",
                "--preserve-commits",
                f.worktree.to_str().unwrap(),
            ],
            0,
        );
        assert_eq!(applied["summary"]["removed"], 1);
        let recovery = applied["results"][0]["evidence"]["recovery_ref"]
            .as_str()
            .unwrap();
        assert_eq!(git(&f.repo, &["rev-parse", recovery]), head);
        assert_eq!(
            git(&f.repo, &["show", &format!("{recovery}:file")]),
            "unpublished committed work"
        );
        assert!(!f.worktree.exists());
        assert!(f.repo.join("file").is_file());
    }
}

#[test]
fn commit_preservation_never_authorizes_file_loss_or_wrong_scope() {
    let f = Fixture::new();
    fs::write(f.worktree.join("evidence"), "keep local files").unwrap();
    let before = git(&f.repo, &["for-each-ref"]);
    let blocked = f.result(
        &[
            "--apply",
            "--preserve-commits",
            f.worktree.to_str().unwrap(),
        ],
        1,
    );
    assert_eq!(blocked["summary"]["blocked"], 1);
    assert_eq!(
        fs::read_to_string(f.worktree.join("evidence")).unwrap(),
        "keep local files"
    );
    assert_eq!(git(&f.repo, &["for-each-ref"]), before);
    let out = f
        .command(&["--apply", "--preserve-commits", f.repo.to_str().unwrap()])
        .arg(&f.worktree)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("not a selected linked worktree"));
    assert_eq!(git(&f.repo, &["for-each-ref"]), before);
    f.unchanged();
}

#[test]
fn pragmatic_cleanup_removes_idle_worktrees_offline_and_preserves_active_work() {
    for format in ["sha1", "sha256"] {
        let f = Fixture::with_object_format(format);
        fs::write(f.worktree.join(".gitignore"), "cache/\n").unwrap();
        git(&f.worktree, &["add", ".gitignore"]);
        git(&f.worktree, &["commit", "-m", "unpublished idle work"]);
        git(&f.worktree, &["checkout", "--detach"]);
        let head = git(&f.worktree, &["rev-parse", "HEAD"]);
        fs::create_dir(f.worktree.join("cache")).unwrap();
        fs::write(f.worktree.join("cache/data"), "ignored cache").unwrap();
        let active = f.temp.path().join("active");
        git(
            &f.repo,
            &["worktree", "add", "-b", "active", active.to_str().unwrap()],
        );
        fs::write(active.join("untracked-work"), "keep this work").unwrap();
        git(&f.repo, &["remote", "remove", "origin"]);
        let run = |apply: bool| {
            let mut c = Command::new(BIN);
            env(&mut c);
            c.args(["git", "worktree", "clean", "--json"]);
            if apply {
                c.arg("--apply");
            }
            c.arg(&f.worktree).arg(&active);
            let out = c.output().unwrap();
            assert_eq!(out.status.code(), Some(0), "{}", text(&out));
            serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap()
        };
        let refs = git(&f.repo, &["for-each-ref"]);
        let preview = run(false);
        assert_eq!(preview["summary"]["would_remove"], 1);
        assert_eq!(preview["summary"]["protected_worktrees"], 1);
        assert_eq!(git(&f.repo, &["for-each-ref"]), refs);
        assert!(f.worktree.join("cache/data").is_file());
        let applied = run(true);
        assert_eq!(applied["summary"]["removed"], 1);
        assert_eq!(applied["summary"]["protected_worktrees"], 1);
        assert!(!f.worktree.exists());
        git(&f.repo, &["branch", "-D", "feature"]);
        let removed = applied["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["status"] == "removed")
            .unwrap();
        let restored = env(&mut Command::new("zsh"))
            .args([
                "-f",
                "-c",
                removed["evidence"]["restore_command"].as_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(restored.status.success(), "{}", text(&restored));
        assert_eq!(git(&f.worktree, &["rev-parse", "HEAD"]), head);
        assert!(!f.worktree.join("cache/data").exists());
        assert_eq!(
            fs::read_to_string(active.join("untracked-work")).unwrap(),
            "keep this work"
        );
        assert_eq!(
            git(
                &f.repo,
                &["rev-parse", &format!("refs/workctl/cleanup/{head}")]
            ),
            head
        );
        assert_eq!(
            git(
                &f.repo,
                &["show", &format!("refs/workctl/cleanup/{head}:.gitignore")]
            ),
            "cache/"
        );
        assert!(f.repo.join("file").is_file());
    }
}

#[test]
fn pragmatic_cleanup_refuses_removal_when_history_cannot_be_saved() {
    let f = Fixture::new();
    let shim = f.shim(
        "case \" $* \" in *' update-ref '*) echo 'recovery write denied' >&2; exit 91;; esac",
    );
    let mut c = Command::new(BIN);
    env(&mut c);
    c.args(["git", "worktree", "clean", "--apply", "--json"])
        .arg(&f.worktree)
        .env(
            "PATH",
            format!("{}:{}", shim.display(), std::env::var("PATH").unwrap()),
        );
    let out = c.output().unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(
        text(&out).contains("recovery write denied"),
        "{}",
        text(&out)
    );
    f.unchanged();
}

#[test]
fn pragmatic_cleanup_protects_tracked_masked_and_late_untracked_work() {
    for kind in ["tracked", "masked", "late untracked"] {
        let f = Fixture::new();
        if kind == "masked" {
            git(&f.worktree, &["update-index", "--skip-worktree", "file"]);
        }
        if kind != "late untracked" {
            fs::write(f.worktree.join("file"), "active tracked work").unwrap();
        }
        let mut c = Command::new(BIN);
        env(&mut c);
        c.args(["git", "worktree", "clean", "--apply", "--json"])
            .arg(&f.worktree);
        if kind == "late untracked" {
            let evidence = f.worktree.join("last-moment-work");
            let shim = f.shim(&format!(
                "case \"$*\" in *'worktree remove'*) printf precious > '{}' ;; esac",
                evidence.display()
            ));
            with_shim(&mut c, &shim);
        }
        let out = c.output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(if kind == "late untracked" { 1 } else { 0 }),
            "{}",
            text(&out)
        );
        let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result["summary"]["removed"], 0);
        if kind != "late untracked" {
            assert_eq!(result["summary"]["protected_worktrees"], 1);
        }
        if kind == "late untracked" {
            assert_eq!(
                fs::read_to_string(f.worktree.join("last-moment-work")).unwrap(),
                "precious"
            );
        } else {
            assert_eq!(
                fs::read_to_string(f.worktree.join("file")).unwrap(),
                "active tracked work"
            );
        }
        f.unchanged();
    }
}

#[test]
fn pragmatic_cleanup_does_not_overwrite_conflicting_or_symbolic_recovery_refs() {
    for symbolic in [false, true] {
        let f = Fixture::new();
        let old = git(&f.repo, &["rev-parse", "HEAD"]);
        fs::write(f.worktree.join("file"), "committed history").unwrap();
        git(&f.worktree, &["add", "file"]);
        git(&f.worktree, &["commit", "-m", "local history"]);
        let head = git(&f.worktree, &["rev-parse", "HEAD"]);
        let recovery = format!("refs/workctl/cleanup/{head}");
        if symbolic {
            git(&f.repo, &["symbolic-ref", &recovery, "refs/heads/feature"]);
        } else {
            git(&f.repo, &["update-ref", &recovery, &old]);
        }
        let before = git(
            &f.repo,
            &[
                "for-each-ref",
                "--format=%(refname) %(objectname) %(symref)",
            ],
        );
        let out = env(&mut Command::new(BIN))
            .args(["git", "worktree", "clean", "--apply", "--json"])
            .arg(&f.worktree)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(text(&out).contains("recovery ref"), "{}", text(&out));
        assert_eq!(
            git(
                &f.repo,
                &[
                    "for-each-ref",
                    "--format=%(refname) %(objectname) %(symref)"
                ]
            ),
            before
        );
        f.unchanged();
    }
}

#[test]
fn pragmatic_cleanup_revalidates_commits_made_while_saving_recovery() {
    let f = Fixture::new();
    let head = git(&f.worktree, &["rev-parse", "HEAD"]);
    let shim = f.shim("case \" $* \" in *' update-ref '*) \"$REAL_GIT\" -C \"$PROBE_WORKTREE\" -c user.name=Probe -c user.email=probe@example.test commit --allow-empty -m concurrent-work >/dev/null || exit $?;; esac");
    let real = Command::new("sh")
        .args(["-c", "command -v git"])
        .output()
        .unwrap();
    let mut c = Command::new(BIN);
    env(&mut c);
    c.args(["git", "worktree", "clean", "--apply", "--json"])
        .arg(&f.worktree)
        .env("REAL_GIT", String::from_utf8(real.stdout).unwrap().trim())
        .env("PROBE_WORKTREE", &f.worktree);
    with_shim(&mut c, &shim);
    let out = c.output().unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(
        text(&out).contains("registration changed during inspection"),
        "{}",
        text(&out)
    );
    assert_ne!(git(&f.worktree, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        git(
            &f.repo,
            &["rev-parse", &format!("refs/workctl/cleanup/{head}")]
        ),
        head
    );
    f.unchanged();
}

#[test]
fn pragmatic_recovery_deadlines_and_cancellation_preserve_work_and_release_locks() {
    for cancel in [false, true] {
        let f = Fixture::new();
        let started = f.temp.path().join("recovery-started");
        let count = f.temp.path().join("attempts");
        let shim = f.shim("case \" $* \" in *' update-ref '*) echo attempt >> \"$PROBE_COUNT\"; touch \"$PROBE_STARTED\"; sleep 30;; esac");
        let mut c = Command::new(BIN);
        env(&mut c);
        c.args([
            "git",
            "worktree",
            "clean",
            "--apply",
            "--json",
            "--timeout",
            if cancel { "30" } else { "4" },
        ])
        .arg(&f.worktree)
        .env("PROBE_STARTED", &started)
        .env("PROBE_COUNT", &count)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
        with_shim(&mut c, &shim);
        let child = c.spawn().unwrap();
        wait(&started);
        if cancel {
            unsafe {
                libc::kill(child.id() as i32, libc::SIGINT);
            }
        }
        let out = child.wait_with_output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(if cancel { 130 } else { 1 }),
            "{}",
            text(&out)
        );
        assert_eq!(fs::read_to_string(count).unwrap(), "attempt\n");
        assert!(git(&f.repo, &["for-each-ref", "refs/workctl/cleanup/"]).is_empty());
        f.unchanged();
        let retry = env(&mut Command::new(BIN))
            .args(["git", "worktree", "clean", "--json"])
            .arg(&f.worktree)
            .output()
            .unwrap();
        assert!(retry.status.success(), "{}", text(&retry));
        f.unchanged();
    }
}
