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
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let repo = root.join("main");
        let remote = root.join("remote.git");
        let worktree = root.join("linked with spaces");
        fs::create_dir(&repo).unwrap();
        git(&repo, &["init", "-b", "main"]);
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
    let f = Fixture::new();
    let bin = f.shim("case \"$*\" in *'worktree remove'*) exit 0;; esac");
    let mut cmd = f.command(&["--apply"]);
    cmd.arg(&f.worktree);
    with_shim(&mut cmd, &bin);
    let out = cmd.output().unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(r["results"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("did not verify"));
    f.unchanged();
}
#[test]
fn deadline_and_cancellation_terminate_groups_without_retrying_cleanup() {
    for cancel in [false, true] {
        let f = Fixture::new();
        let started = f.temp.path().join("started");
        let escaped = f.temp.path().join("escaped");
        let count = f.temp.path().join("count");
        let bin=f.shim(&format!("for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then echo attempt >> '{}'; touch '{}'; (sleep 3; touch '{}') & sleep 30; fi; done",count.display(),started.display(),escaped.display()));
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
