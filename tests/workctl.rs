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
    fn command(&self, extra: &[&str]) -> Command {
        let mut c = Command::new(BIN);
        env(&mut c);
        c.current_dir(&self.repo)
            .args(["git", "worktree", "clean", "--json"])
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
        vec!["git", "worktree", "clean", "--apply", "--json", "--verbose"],
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
    let mut child = env(&mut Command::new(BIN))
        .args(["git", "reset"])
        .arg(&f.repo)
        .env("TMPDIR", &runtime)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stderr.take());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(String::from_utf8_lossy(&out.stdout).contains("Git checkout reset"));
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
        let mut cmd = f.command(&["--apply", "--timeout", if cancel { "30" } else { "1" }]);
        cmd.arg(&f.worktree);
        with_shim(&mut cmd, &bin);
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
    let out = Command::new("zsh")
        .args(["-f", "-c", "fpath=(\"$HOME/.local/share/zsh/site-functions\" $fpath); autoload -Uz compinit compaudit; compaudit || exit 1; compinit -D; rehash; [[ ${_comps[workctl]} == _workctl ]] && autoload +X _workctl && ! whence reset_to_origin && workctl doctor --json"])
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
fn cleanup_closed_output_and_submodules_never_authorize_removal() {
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
    let r = f.result(
        &["--apply", "--discard-local", f.worktree.to_str().unwrap()],
        1,
    );
    assert!(r["results"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("submodules"));
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
