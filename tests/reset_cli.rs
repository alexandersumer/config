//! Acceptance tests exercise the public compiled CLI and real disposable Git repos.
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::{
    ffi::OsStringExt,
    fs::{symlink, PermissionsExt},
};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const BIN: &str = env!("CARGO_BIN_EXE_workctl");
struct Fixture {
    temp: TempDir,
    seed: PathBuf,
    origin: PathBuf,
    workspace: PathBuf,
    repo: PathBuf,
    old: String,
    new: String,
}
fn env(cmd: &mut Command) -> &mut Command {
    cmd.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_CONFIG_COUNT")
}
fn git(path: &Path, args: &[&str]) -> String {
    let out = env(Command::new("git").current_dir(path))
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let seed = temp.path().join("seed");
        let origin = temp.path().join("origin.git");
        let workspace = temp.path().join("workspace");
        let repo = workspace.join("a repo");
        fs::create_dir(&seed).unwrap();
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(temp.path().join("runtime")).unwrap();
        git(&seed, &["init", "-b", "main"]);
        fs::write(seed.join("file"), "initial\n").unwrap();
        git(&seed, &["add", "."]);
        git(&seed, &["commit", "-m", "initial"]);
        git(
            temp.path(),
            &["clone", "--bare", origin_arg(&seed), origin_arg(&origin)],
        );
        git(&seed, &["remote", "add", "origin", origin_arg(&origin)]);
        git(
            &workspace,
            &["clone", origin_arg(&origin), origin_arg(&repo)],
        );
        let old = git(&repo, &["rev-parse", "HEAD"]);
        fs::write(seed.join("file"), "updated\n").unwrap();
        git(&seed, &["commit", "-am", "updated"]);
        git(&seed, &["push", "origin", "main"]);
        let new = git(&seed, &["rev-parse", "HEAD"]);
        Self {
            temp,
            seed,
            origin,
            workspace,
            repo,
            old,
            new,
        }
    }
    fn cli(&self, path: &Path, args: &[&OsStr]) -> Command {
        let mut cmd = Command::new(BIN);
        env(&mut cmd);
        cmd.env("TMPDIR", self.temp.path().join("runtime"));
        cmd.args(["git", "reset"])
            .arg("--keep-logs")
            .arg("--attempts")
            .arg("1")
            .args(args)
            .current_dir(path);
        cmd
    }
    fn run(&self, path: &Path, args: &[&str]) -> Output {
        self.cli(path, &args.iter().map(OsStr::new).collect::<Vec<_>>())
            .output()
            .unwrap()
    }
    fn clone(&self, name: &str) -> PathBuf {
        let path = self.workspace.join(name);
        git(
            &self.workspace,
            &["clone", origin_arg(&self.origin), origin_arg(&path)],
        );
        path
    }
    fn push_file(&self, name: &str) {
        fs::write(self.seed.join(name), "remote").unwrap();
        git(&self.seed, &["add", name]);
        git(&self.seed, &["commit", "-m", "new file"]);
        git(&self.seed, &["push", "origin", "main"]);
    }
    fn shim(&self, script: &str) -> PathBuf {
        let dir = self.temp.path().join("bin");
        fs::create_dir_all(&dir).unwrap();
        let real = String::from_utf8(
            Command::new("sh")
                .args(["-c", "command -v git"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        fs::write(
            dir.join("git"),
            format!("#!/bin/sh\n{script}\nexec '{}' \"$@\"\n", real.trim()),
        )
        .unwrap();
        fs::set_permissions(dir.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
        dir
    }
    fn with_shim(&self, cmd: &mut Command, dir: &Path) {
        cmd.env(
            "PATH",
            format!("{}:{}", dir.display(), std::env::var("PATH").unwrap()),
        );
    }
}
fn origin_arg(p: &Path) -> &str {
    p.to_str().unwrap()
}
fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}
fn success(out: &Output) {
    assert!(out.status.success(), "{}", text(out));
}
fn failure(out: &Output, fragment: &str) {
    assert_eq!(out.status.code(), Some(1), "{}", text(out));
    assert!(
        text(out)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains(fragment),
        "{}",
        text(out)
    );
}
fn logs(out: &Output) -> PathBuf {
    PathBuf::from(
        String::from_utf8_lossy(&out.stderr)
            .lines()
            .find_map(|l| l.strip_prefix("Logs: "))
            .unwrap(),
    )
}
fn wait_marker(path: &Path) {
    let until = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(
            Instant::now() < until,
            "marker not created: {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(10));
    }
}
fn wait_child(mut child: Child) -> Output {
    let until = Instant::now() + Duration::from_secs(15);
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= until {
            let _ = child.kill();
            panic!("CLI failed to terminate");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn direct_current_and_discovered_targets_share_backup_and_preservation() {
    for mode in [0, 1, 2, 3] {
        let f = Fixture::new();
        git(&f.repo, &["branch", "feature"]);
        fs::write(f.repo.join("notes"), "keep").unwrap();
        git(&f.repo, &["switch", "feature"]);
        let out = match mode {
            0 => f.run(&f.repo, &[]),
            1 => f.run(&f.workspace, &[origin_arg(&f.repo)]),
            2 => f.run(&f.workspace, &["."]),
            _ => {
                fs::create_dir(f.repo.join("inside")).unwrap();
                f.run(&f.repo.join("inside"), &[])
            }
        };
        success(&out);
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
        assert_eq!(git(&f.repo, &["rev-parse", "feature"]), f.old);
        assert_eq!(fs::read_to_string(f.repo.join("notes")).unwrap(), "keep");
        assert!(git(
            &f.repo,
            &[
                "for-each-ref",
                "--format=%(objectname)",
                "refs/home-reset-backups"
            ]
        )
        .contains(&f.old));
        let records: serde_json::Value =
            serde_json::from_slice(&fs::read(logs(&out).join("results.json")).unwrap()).unwrap();
        assert_eq!(records[0]["attempts"], 1);
    }
}

#[test]
fn dirty_index_and_worktree_refuse_before_fetch_while_other_repos_finish() {
    for staged in [false, true] {
        let f = Fixture::new();
        let other = f.clone("other");
        fs::write(f.repo.join("file"), "precious").unwrap();
        if staged {
            git(&f.repo, &["add", "file"]);
        }
        let out = f.run(&f.workspace, &[]);
        failure(&out, "Tracked changes");
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
        assert_eq!(git(&f.repo, &["rev-parse", "origin/main"]), f.old);
        assert_eq!(git(&other, &["rev-parse", "HEAD"]), f.new);
        assert_eq!(fs::read_to_string(f.repo.join("file")).unwrap(), "precious");
    }
}

#[test]
fn untracked_ignored_and_file_directory_collisions_are_preserved() {
    for kind in [0, 1, 2, 3] {
        let f = Fixture::new();
        if kind == 2 {
            fs::create_dir(f.seed.join("collision")).unwrap();
            f.push_file("collision/remote");
            fs::write(f.repo.join("collision"), "precious").unwrap();
        } else {
            f.push_file("collision");
            if kind == 3 {
                fs::create_dir(f.repo.join("collision")).unwrap();
                fs::write(f.repo.join("collision/local"), "precious").unwrap();
            } else {
                fs::write(f.repo.join("collision"), "precious").unwrap();
            }
        }
        if kind == 1 {
            fs::write(f.repo.join(".git/info/exclude"), "collision\n").unwrap();
        }
        let out = f.run(&f.repo, &[]);
        failure(&out, "would be overwritten");
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
        let precious = if kind == 3 {
            f.repo.join("collision/local")
        } else {
            f.repo.join("collision")
        };
        assert_eq!(fs::read_to_string(precious).unwrap(), "precious");
    }
}

#[test]
fn existing_local_target_tree_is_checked_before_switch() {
    let f = Fixture::new();
    fs::write(f.repo.join("local-only"), "tracked").unwrap();
    git(&f.repo, &["add", "."]);
    git(&f.repo, &["commit", "-m", "local main"]);
    let tip = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["switch", "-c", "feature", &f.old]);
    fs::write(f.repo.join(".git/info/exclude"), "local-only\n").unwrap();
    fs::write(f.repo.join("local-only"), "precious").unwrap();
    failure(&f.run(&f.repo, &[]), "would be overwritten");
    assert_eq!(git(&f.repo, &["rev-parse", "main"]), tip);
    assert_eq!(git(&f.repo, &["branch", "--show-current"]), "feature");
}

#[test]
fn hooks_cannot_have_their_changes_destroyed_and_partial_stage_is_reported() {
    let f = Fixture::new();
    let hook = f.repo.join(".git/hooks/post-checkout");
    fs::write(&hook, "#!/bin/sh\nprintf precious > file\n").unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    let out = f.run(&f.repo, &[]);
    failure(&out, "post-switch safety check");
    assert_eq!(fs::read_to_string(f.repo.join("file")).unwrap(), "precious");
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    assert!(fs::read_to_string(logs(&out).join("0001.log"))
        .unwrap()
        .contains("State: switched"));
}

#[test]
fn unfinished_operations_and_existing_git_locks_are_not_repaired() {
    for marker in [
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "BISECT_LOG",
        "rebase-merge",
        "rebase-apply",
        "sequencer",
    ] {
        let f = Fixture::new();
        let path = f.repo.join(".git").join(marker);
        if marker.starts_with("rebase") || marker == "sequencer" {
            fs::create_dir(&path).unwrap();
        } else {
            fs::write(&path, &f.old).unwrap();
        }
        failure(&f.run(&f.repo, &[]), "operation in progress");
        assert!(path.exists());
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    }
    let f = Fixture::new();
    let lock = f.repo.join(".git/refs/remotes/origin/main.lock");
    fs::write(&lock, "active lock").unwrap();
    failure(&f.run(&f.repo, &[]), "fetch");
    assert_eq!(fs::read_to_string(lock).unwrap(), "active lock");
}

#[test]
fn linked_worktrees_are_skipped_and_explicit_worktrees_refused() {
    let f = Fixture::new();
    let linked = f.workspace.join("linked");
    git(
        &f.repo,
        &["worktree", "add", "-b", "feature", origin_arg(&linked)],
    );
    fs::write(linked.join("file"), "precious").unwrap();
    fs::write(linked.join("notes"), "keep").unwrap();
    let out = f.run(&f.workspace, &[]);
    success(&out);
    assert!(text(&out).contains("Excluded       1 linked worktrees"));
    assert_eq!(fs::read_to_string(linked.join("file")).unwrap(), "precious");
    assert_eq!(git(&linked, &["rev-parse", "HEAD"]), f.old);
    failure(
        &f.run(
            &f.workspace,
            &[origin_arg(&f.workspace), origin_arg(&linked)],
        ),
        "explicit linked worktree",
    );
    let f = Fixture::new();
    git(&f.repo, &["switch", "-c", "feature"]);
    let linked = f.workspace.join("default-linked");
    git(&f.repo, &["worktree", "add", origin_arg(&linked), "main"]);
    failure(&f.run(&f.repo, &[]), "switch");
    assert_eq!(git(&f.repo, &["branch", "--show-current"]), "feature");
    assert_eq!(git(&linked, &["rev-parse", "HEAD"]), f.old);
}

#[test]
fn separate_git_directory_is_a_primary_checkout() {
    let f = Fixture::new();
    let separate = f.workspace.join("separate");
    fs::create_dir(&separate).unwrap();
    let metadata = f.temp.path().join("metadata");
    git(
        &separate,
        &[
            "init",
            "-b",
            "main",
            "--separate-git-dir",
            origin_arg(&metadata),
        ],
    );
    git(
        &separate,
        &["remote", "add", "origin", origin_arg(&f.origin)],
    );
    success(&f.run(&separate, &[]));
    assert_eq!(git(&separate, &["rev-parse", "HEAD"]), f.new);
}

#[test]
fn discovery_deduplicates_without_symlink_walks_or_nested_resets() {
    let f = Fixture::new();
    let group = f.workspace.join("group");
    fs::create_dir(&group).unwrap();
    let grouped = group.join("repo");
    git(
        &group,
        &["clone", origin_arg(&f.origin), origin_arg(&grouped)],
    );
    symlink(&f.workspace, group.join("loop")).unwrap();
    symlink(&f.repo, f.workspace.join("alias")).unwrap();
    git(&f.repo, &["clone", origin_arg(&f.origin), "nested"]);
    let out = f.run(&f.workspace, &["--list", ".", origin_arg(&f.repo), "."]);
    success(&out);
    assert_eq!(String::from_utf8_lossy(&out.stdout).lines().count(), 2);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    assert!(!f.repo.join(".git/repo-batch.lock").exists());
    success(&f.run(&f.workspace, &[".", origin_arg(&f.repo), "."]));
    assert_eq!(git(&f.repo.join("nested"), &["rev-parse", "HEAD"]), f.new);
}

#[test]
fn corrupt_discovery_and_invalid_late_arguments_prevent_all_resets() {
    let f = Fixture::new();
    let corrupt = f.workspace.join("zz-corrupt");
    fs::create_dir(&corrupt).unwrap();
    fs::write(corrupt.join(".git"), "invalid").unwrap();
    failure(&f.run(&f.workspace, &[]), "corrupt");
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    failure(
        &f.run(&f.workspace, &[origin_arg(&f.repo), "missing"]),
        "missing",
    );
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    for args in [
        ["--jobs", "0"],
        ["--attempts", "11"],
        ["--remote", "-danger"],
    ] {
        assert_eq!(f.run(&f.repo, &args).status.code(), Some(2));
    }
}

#[test]
fn default_rename_and_explicit_remote_branch_are_authoritative() {
    let f = Fixture::new();
    git(&f.seed, &["switch", "-c", "trunk"]);
    fs::write(f.seed.join("file"), "origin trunk tip\n").unwrap();
    git(&f.seed, &["commit", "-am", "advance trunk independently"]);
    let trunk = git(&f.seed, &["rev-parse", "HEAD"]);
    git(&f.seed, &["push", "origin", "trunk"]);
    git(&f.origin, &["symbolic-ref", "HEAD", "refs/heads/trunk"]);
    success(&f.run(&f.repo, &[]));
    assert_eq!(git(&f.repo, &["branch", "--show-current"]), "trunk");
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), trunk);
    assert_eq!(git(&f.repo, &["rev-parse", "main"]), f.old);

    let upstream = f.temp.path().join("upstream.git");
    git(
        f.temp.path(),
        &[
            "clone",
            "--bare",
            origin_arg(&f.seed),
            origin_arg(&upstream),
        ],
    );
    git(
        &f.seed,
        &["remote", "add", "upstream", origin_arg(&upstream)],
    );
    git(&f.seed, &["switch", "main"]);
    fs::write(f.seed.join("file"), "upstream main tip\n").unwrap();
    git(
        &f.seed,
        &["commit", "-am", "advance upstream main independently"],
    );
    git(&f.seed, &["push", "upstream", "main"]);
    let upstream_main = git(&f.seed, &["rev-parse", "HEAD"]);
    git(
        &f.repo,
        &["remote", "add", "upstream", origin_arg(&upstream)],
    );
    success(&f.run(&f.repo, &["--remote", "upstream", "--branch", "main"]));
    assert_eq!(git(&f.repo, &["branch", "--show-current"]), "main");
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), upstream_main);
    assert_eq!(git(&f.repo, &["rev-parse", "trunk"]), trunk);
    assert_eq!(git(&f.repo, &["rev-parse", "origin/main"]), f.new);
    assert_eq!(
        git(
            &f.repo,
            &["rev-parse", "--symbolic-full-name", "main@{upstream}"]
        ),
        "refs/remotes/upstream/main"
    );
    git(&f.origin, &["symbolic-ref", "HEAD", "refs/heads/missing"]);
    failure(&f.run(&f.repo, &[]), "default branch");
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), upstream_main);
}

#[test]
fn custom_fetch_mapping_is_respected_and_excluded_targets_are_refused() {
    let f = Fixture::new();
    git(
        &f.repo,
        &[
            "config",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/custom/*",
        ],
    );
    success(&f.run(&f.repo, &[]));
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
    assert_eq!(
        git(
            &f.repo,
            &["rev-parse", "--symbolic-full-name", "main@{upstream}"]
        ),
        "refs/remotes/custom/main"
    );
    git(
        &f.repo,
        &["config", "--add", "remote.origin.fetch", "^refs/heads/main"],
    );
    let before = git(&f.repo, &["config", "--get-all", "remote.origin.fetch"]);
    failure(&f.run(&f.repo, &[]), "excluded");
    assert_eq!(
        git(&f.repo, &["config", "--get-all", "remote.origin.fetch"]),
        before
    );
    let f = Fixture::new();
    git(
        &f.repo,
        &[
            "config",
            "remote.origin.fetch",
            "+refs/heads/other:refs/remotes/origin/other",
        ],
    );
    failure(&f.run(&f.repo, &[]), "fetch mapping");
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
}

#[test]
fn ignored_files_and_submodule_contents_are_preserved() {
    let f = Fixture::new();
    git(
        &f.seed,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            origin_arg(&f.origin),
            "module",
        ],
    );
    git(&f.seed, &["commit", "-am", "module"]);
    git(&f.seed, &["push", "origin", "main"]);
    git(&f.repo, &["fetch", "origin"]);
    git(&f.repo, &["reset", "--hard", "origin/main"]);
    git(
        &f.repo,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "update",
            "--init",
        ],
    );
    git(&f.repo, &["config", "submodule.recurse", "true"]);
    let module = f.repo.join("module");
    let before = git(&module, &["rev-parse", "HEAD"]);
    fs::write(module.join("ignored"), "precious").unwrap();
    let dir = git(&module, &["rev-parse", "--absolute-git-dir"]);
    fs::write(Path::new(&dir).join("info/exclude"), "ignored\n").unwrap();
    success(&f.run(&f.repo, &[]));
    assert_eq!(git(&module, &["rev-parse", "HEAD"]), before);
    assert_eq!(
        fs::read_to_string(module.join("ignored")).unwrap(),
        "precious"
    );
}

#[test]
fn paths_with_newlines_and_non_utf8_bytes_are_accepted_and_safely_displayed() {
    let f = Fixture::new();
    let bytes = if cfg!(target_os = "linux") {
        b"odd\n\xff".to_vec()
    } else {
        "odd\n unicode-é".as_bytes().to_vec()
    };
    let odd = f.workspace.join(OsString::from_vec(bytes));
    fs::rename(&f.repo, &odd).unwrap();
    let out = f.cli(&f.workspace, &[odd.as_os_str()]).output().unwrap();
    success(&out);
    assert_eq!(git(&odd, &["rev-parse", "HEAD"]), f.new);
    let out = f.run(&f.workspace, &["--list"]);
    success(&out);
    assert!(text(&out).contains("\\u{a}"));
    assert_eq!(String::from_utf8_lossy(&out.stdout).lines().count(), 1);
}

#[test]
fn fetch_timeout_cancellation_and_normal_completion_reap_descendants() {
    for mode in ["timeout", "cancel", "normal"] {
        let f = Fixture::new();
        let marker = f.temp.path().join("started");
        let escaped = f.temp.path().join("escaped");
        let script = format!("for arg in \"$@\"; do\n if [ \"$arg\" = fetch ]; then\n  (sleep 5; touch '{}') &\n  echo $$ > '{}'\n  {}\n fi\ndone", escaped.display(), marker.display(), if mode == "normal" { ":" } else { "sleep 20" });
        let dir = f.shim(&script);
        let timeout = if mode == "normal" { "10" } else { "1" };
        let mut cmd = f.cli(&f.repo, &[OsStr::new("--timeout"), OsStr::new(timeout)]);
        f.with_shim(&mut cmd, &dir);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let child = cmd.spawn().unwrap();
        wait_marker(&marker);
        if mode == "cancel" {
            unsafe {
                libc::kill(child.id() as i32, libc::SIGINT);
            }
        }
        let out = wait_child(child);
        if mode == "normal" {
            success(&out);
        } else {
            assert_eq!(
                out.status.code(),
                Some(if mode == "cancel" { 130 } else { 1 }),
                "{}",
                text(&out)
            );
            assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
        }
        let results: serde_json::Value =
            serde_json::from_slice(&fs::read(logs(&out).join("results.json")).unwrap()).unwrap();
        let record = &results[0];
        assert_eq!(
            record["path"],
            f.repo.canonicalize().unwrap().to_string_lossy().as_ref()
        );
        assert_eq!(
            record["code"],
            match mode {
                "timeout" => 124,
                "cancel" => 130,
                _ => 0,
            }
        );
        assert_eq!(record["attempts"], 1);
        assert_eq!(record["target"], format!("origin/main ({})", &f.new[..12]));
        if mode != "normal" {
            assert!(record["detail"].as_str().unwrap().contains("fetch"));
            assert!(record["backups"].as_array().unwrap().is_empty());
        } else {
            assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
        }
        thread::sleep(Duration::from_secs(5));
        assert!(!escaped.exists(), "descendant outlived {mode}");
    }
}

#[test]
fn overlapping_invocations_refuse_busy_repository() {
    let f = Fixture::new();
    let marker = f.temp.path().join("started");
    let dir = f.shim(&format!(
        "for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then echo $$ > '{}'; sleep 20; fi; done",
        marker.display()
    ));
    let mut cmd = f.cli(&f.repo, &[]);
    f.with_shim(&mut cmd, &dir);
    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_marker(&marker);
    failure(&f.run(&f.repo, &[]), "busy");
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }
    assert_eq!(wait_child(child).status.code(), Some(130));
    success(&f.run(&f.repo, &[]));
}

#[test]
fn transient_failures_retry_but_authentication_failures_do_not() {
    for transient in [true, false] {
        let f = Fixture::new();
        let count = f.temp.path().join("count");
        let dir = f.shim(&format!("for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then n=$(cat '{}' 2>/dev/null || echo 0); n=$((n+1)); echo $n > '{}'; if [ $n -lt 2 ]; then echo '{}' >&2; exit 1; fi; fi; done", count.display(), count.display(), if transient {"Connection reset"} else {"Permission denied (publickey)"}));
        let mut cmd = f.cli(&f.repo, &[OsStr::new("--attempts"), OsStr::new("2")]);
        f.with_shim(&mut cmd, &dir);
        let out = cmd.output().unwrap();
        if transient {
            success(&out);
            assert_eq!(fs::read_to_string(count).unwrap().trim(), "2");
        } else {
            failure(&out, "Permission denied");
            assert_eq!(fs::read_to_string(count).unwrap().trim(), "1");
        }
    }
}

#[test]
fn installed_name_dispatches_without_config_checkout_or_shell() {
    let f = Fixture::new();
    let executable = f.temp.path().join("workctl");
    fs::copy(BIN, &executable).unwrap();
    let out = env(Command::new(executable).current_dir(&f.repo))
        .args(["git", "reset"])
        .env("HOME", f.temp.path().join("empty-home"))
        .output()
        .unwrap();
    success(&out);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
}

#[test]
fn empty_container_succeeds_without_logs_or_mutation() {
    let f = Fixture::new();
    let empty = f.temp.path().join("empty");
    fs::create_dir(&empty).unwrap();
    let out = f.run(&empty, &[]);
    success(&out);
    assert!(text(&out).contains("No repositories"));
    assert!(!text(&out).contains("Logs:"));
}

#[test]
fn multiple_roots_use_bounded_parallel_workers_and_quiet_success_output() {
    let f = Fixture::new();
    let mut repos = vec![f.repo.clone()];
    for name in ["b", "c", "d", "e"] {
        repos.push(f.clone(name));
    }
    let other = f.temp.path().join("other-root");
    fs::create_dir(&other).unwrap();
    git(&other, &["clone", origin_arg(&f.origin), "last"]);
    repos.push(other.join("last"));
    for repo in &repos {
        git(repo, &["reset", "--hard", &f.old]);
    }
    let trace = f.temp.path().join("trace");
    let dir = f.shim(&format!("for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then printf 'start\\n' >> '{}'; sleep 0.3; printf 'end\\n' >> '{}'; fi; done", trace.display(), trace.display()));
    let mut cmd = f.cli(
        f.temp.path(),
        &[
            OsStr::new("--jobs"),
            OsStr::new("3"),
            f.workspace.as_os_str(),
            other.as_os_str(),
        ],
    );
    f.with_shim(&mut cmd, &dir);
    let out = cmd.output().unwrap();
    success(&out);
    let mut active = 0;
    let mut peak = 0;
    for event in fs::read_to_string(trace).unwrap().lines() {
        if event == "start" {
            active += 1;
        } else {
            active -= 1;
        }
        peak = peak.max(active);
        assert!(active <= 3);
    }
    assert_eq!(active, 0);
    assert!(peak >= 2, "execution was sequential");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("Git checkout reset\n"));
    assert!(stdout.contains("Checkouts      6"));
    assert!(stdout.contains("Workers        3"));
    assert!(stdout.contains("\nCompleted\n"));
    assert!(stdout.contains("6 succeeded"));
    assert!(!stdout.contains("  Reset "));
    assert!(!stdout.contains('\x1b'));
    let records: serde_json::Value =
        serde_json::from_slice(&fs::read(logs(&out).join("results.json")).unwrap()).unwrap();
    let records = records.as_array().unwrap();
    assert_eq!(records.len(), 6);
    let mut repos: Vec<_> = repos
        .into_iter()
        .map(|p| p.canonicalize().unwrap())
        .collect();
    repos.sort();
    let reported: Vec<_> = records
        .iter()
        .map(|r| PathBuf::from(r["path"].as_str().unwrap()))
        .collect();
    assert_eq!(reported, repos);
    for (record, repo) in records.iter().zip(&repos) {
        assert_eq!(git(repo, &["rev-parse", "HEAD"]), f.new);
        assert_eq!(git(repo, &["symbolic-ref", "--short", "HEAD"]), "main");
        assert_eq!(
            git(
                repo,
                &["rev-parse", "--symbolic-full-name", "main@{upstream}"]
            ),
            "refs/remotes/origin/main"
        );
        assert_eq!(record["code"], 0);
        assert_eq!(record["attempts"], 1);
        assert_eq!(record["target"], format!("origin/main ({})", &f.new[..12]));
        let backups = record["backups"].as_array().unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(
            git(repo, &["rev-parse", backups[0].as_str().unwrap()]),
            f.old
        );
    }
}

#[test]
fn failure_is_reported_before_a_slower_earlier_repository_finishes() {
    let f = Fixture::new();
    let dirty = f.clone("zz-dirty");
    fs::write(dirty.join("file"), "precious").unwrap();
    let started = f.temp.path().join("started");
    let done = f.temp.path().join("done");
    let dir = f.shim(&format!("for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then touch '{}'; sleep 3; touch '{}'; fi; done", started.display(), done.display()));
    let output = f.temp.path().join("output");
    let mut cmd = f.cli(&f.workspace, &[]);
    f.with_shim(&mut cmd, &dir);
    let child = cmd
        .stdout(Stdio::piped())
        .stderr(fs::File::create(&output).unwrap())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let text = fs::read_to_string(&output).unwrap();
        if text.contains("Failed:") && text.contains("zz-dirty") {
            break;
        }
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        !done.exists(),
        "failure was withheld until the slow job completed"
    );
    assert_eq!(wait_child(child).status.code(), Some(1));
}

#[test]
fn broken_output_cancels_running_workers_instead_of_leaving_fetches() {
    let f = Fixture::new();
    let marker = f.temp.path().join("started");
    let escaped = f.temp.path().join("escaped");
    let dir = f.shim(&format!("for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then touch '{}'; (sleep 3; touch '{}') & sleep 20; fi; done", marker.display(), escaped.display()));
    // /dev/full causes a reporting error as soon as the command writes its header.
    // No worker is allowed to launch after that error.
    if Path::new("/dev/full").exists() {
        let mut cmd = f.cli(&f.repo, &[]);
        f.with_shim(&mut cmd, &dir);
        let out = cmd
            .stdout(
                fs::OpenOptions::new()
                    .write(true)
                    .open("/dev/full")
                    .unwrap(),
            )
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1));
        assert!(!marker.exists());
        assert!(!escaped.exists());
    } else {
        // macOS has no /dev/full. A closed pipe proves the same launch boundary.
        let mut cmd = f.cli(&f.repo, &[]);
        f.with_shim(&mut cmd, &dir);
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        drop(child.stdout.take());
        let out = wait_child(child);
        assert_eq!(out.status.code(), Some(1));
        assert!(!marker.exists());
    }
}

#[test]
fn case_collisions_never_trigger_implicit_ref_or_config_repair() {
    let f = Fixture::new();
    git(
        &f.origin,
        &["update-ref", "refs/heads/NOISSUE/topic", &f.old],
    );
    git(&f.origin, &["pack-refs", "--all"]);
    git(
        &f.origin,
        &["update-ref", "refs/heads/noissue/topic", &f.new],
    );
    git(&f.origin, &["pack-refs", "--all"]);
    let before = git(&f.repo, &["config", "--get-all", "remote.origin.fetch"]);
    let out = f.run(&f.repo, &[]);
    let probe = f.repo.join(".git/case-probe");
    fs::write(&probe, "probe").unwrap();
    let insensitive = f.repo.join(".git/CASE-PROBE").exists();
    fs::remove_file(probe).unwrap();
    if insensitive {
        failure(&out, "fetch");
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    } else {
        success(&out);
    }
    assert_eq!(
        git(&f.repo, &["config", "--get-all", "remote.origin.fetch"]),
        before
    );
    assert_eq!(
        git(&f.origin, &["rev-parse", "refs/heads/NOISSUE/topic"]),
        f.old
    );
    assert_eq!(
        git(&f.origin, &["rev-parse", "refs/heads/noissue/topic"]),
        f.new
    );
}

#[test]
fn cancellation_interrupts_retry_backoff_and_cancels_queued_repositories() {
    let f = Fixture::new();
    f.clone("queued");
    let count = f.temp.path().join("count");
    let dir=f.shim(&format!("for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then printf attempt >> '{}'; echo 'Connection reset' >&2; exit 1; fi; done",count.display()));
    let output = f.temp.path().join("output");
    let mut cmd = f.cli(
        &f.workspace,
        &[
            OsStr::new("--jobs"),
            OsStr::new("1"),
            OsStr::new("--attempts"),
            OsStr::new("3"),
        ],
    );
    f.with_shim(&mut cmd, &dir);
    let child = cmd
        .stdout(Stdio::piped())
        .stderr(fs::File::create(&output).unwrap())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let text = fs::read_to_string(&output).unwrap();
        if let Some(dir) = text.lines().find_map(|line| line.strip_prefix("Logs: ")) {
            if fs::read_to_string(Path::new(dir).join("0001.log"))
                .unwrap_or_default()
                .contains("retrying in")
            {
                break;
            }
        }
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(10));
    }
    let start = Instant::now();
    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }
    let out = wait_child(child);
    assert_eq!(out.status.code(), Some(130));
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_eq!(fs::read_to_string(count).unwrap(), "attempt");
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    let stdout = fs::read_to_string(output).unwrap();
    let directory = stdout
        .lines()
        .find_map(|line| line.strip_prefix("Logs: "))
        .unwrap();
    let records: serde_json::Value =
        serde_json::from_slice(&fs::read(Path::new(directory).join("results.json")).unwrap())
            .unwrap();
    assert_eq!(records[1]["attempts"], 0);
    assert!(Path::new(records[1]["log"].as_str().unwrap()).is_file());
}

#[test]
fn hooks_that_create_commits_are_preserved_and_git_action_output_is_logged() {
    let f = Fixture::new();
    let hook = f.repo.join(".git/hooks/post-checkout");
    fs::write(&hook,"#!/bin/sh\nprintf precious > file\ngit add file\ngit commit -m 'hook work'\nprintf 'hook output\\n'\n").unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    let out = f.run(&f.repo, &["--verbose"]);
    failure(&out, "Target branch tip changed");
    let head = git(&f.repo, &["rev-parse", "HEAD"]);
    assert_ne!(head, f.old);
    assert_ne!(head, f.new);
    assert_eq!(fs::read_to_string(f.repo.join("file")).unwrap(), "precious");
    assert!(text(&out).contains("hook output"));
}

#[test]
fn fetch_mappings_cannot_replace_unrelated_local_branches() {
    let f = Fixture::new();
    git(&f.repo, &["branch", "feature"]);
    git(
        &f.repo,
        &[
            "config",
            "--add",
            "remote.origin.fetch",
            "+refs/heads/main:refs/heads/feature",
        ],
    );
    let before = git(&f.repo, &["config", "--get-all", "remote.origin.fetch"]);
    failure(&f.run(&f.repo, &[]), "replace local branches");
    assert_eq!(git(&f.repo, &["rev-parse", "feature"]), f.old);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    assert_eq!(
        git(&f.repo, &["config", "--get-all", "remote.origin.fetch"]),
        before
    );
}

#[test]
fn remote_movement_between_advertisement_and_fetch_refuses_stale_reset() {
    let f = Fixture::new();
    let real = String::from_utf8(
        Command::new("sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let dir=f.shim(&format!("for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then '{}' --git-dir '{}' update-ref refs/heads/main '{}'; fi; done",real.trim(),f.origin.display(),f.old));
    let mut cmd = f.cli(&f.repo, &[]);
    f.with_shim(&mut cmd, &dir);
    failure(&cmd.output().unwrap(), "Target was not fetched");
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
}

#[test]
fn output_failure_after_workers_start_cancels_their_process_groups() {
    let f = Fixture::new();
    let dirty = f.clone("zz-dirty");
    fs::write(dirty.join("file"), "dirty").unwrap();
    let marker = f.temp.path().join("started");
    let escaped = f.temp.path().join("escaped");
    let dir=f.shim(&format!("case \"$PWD\" in *zz-dirty) case \"$*\" in *status*) sleep 2;; esac;; esac\nfor arg in \"$@\"; do if [ \"$arg\" = fetch ]; then touch '{}'; (sleep 4; touch '{}') & sleep 20; fi; done",marker.display(),escaped.display()));
    let mut cmd = f.cli(&f.workspace, &[]);
    f.with_shim(&mut cmd, &dir);
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_marker(&marker);
    drop(child.stderr.take());
    let out = wait_child(child);
    assert_eq!(out.status.code(), Some(1));
    thread::sleep(Duration::from_secs(4));
    assert!(!escaped.exists());
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
}

#[test]
fn directory_named_like_the_subcommand_is_not_reinterpreted() {
    let f = Fixture::new();
    let repo = f.workspace.join("reset-to-origin");
    fs::rename(&f.repo, &repo).unwrap();
    success(&f.run(&f.workspace, &["reset-to-origin"]));
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), f.new);
}

#[test]
fn public_diagnostics_escape_terminal_controls_and_closed_help_output_is_an_error() {
    let f = Fixture::new();
    let corrupt = f.workspace.join("bad\x1b[31m-name");
    fs::create_dir(&corrupt).unwrap();
    fs::write(corrupt.join(".git"), "invalid").unwrap();
    let out = f.run(&f.workspace, &[]);
    failure(&out, "corrupt");
    assert!(!text(&out).contains('\x1b'));
    assert!(text(&out).contains("\\u{1b}"));
    let mut child = f
        .cli(&f.repo, &[OsStr::new("--help")])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    assert_eq!(wait_child(child).status.code(), Some(1));
}

#[test]
fn clap_help_version_and_invalid_usage_do_not_touch_repositories() {
    let f = Fixture::new();
    for arg in ["-h", "--help", "-V", "--version"] {
        let out = f.run(&f.repo, &[arg]);
        success(&out);
        assert!(out.stderr.is_empty());
        assert!(String::from_utf8_lossy(&out.stdout).contains("workctl"));
        if arg == "--help" {
            let help = text(&out);
            for expected in ["Examples:", "--jobs", "--version", "Exit codes:", "--list"] {
                assert!(help.contains(expected), "missing {expected}: {help}");
            }
        }
    }
    for args in [
        vec!["---help"],
        vec!["--jbos=4"],
        vec!["--jobs=33"],
        vec!["--attempts=0"],
        vec!["--timeout=86401"],
        vec!["--jobs"],
        vec!["--remote="],
        vec!["--branch=-danger"],
        vec!["--jobs=2", "--unknown"],
        vec!["--unknown\x1b[31m"],
    ] {
        let out = f.run(&f.repo, &args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", text(&out));
        assert!(out.stdout.is_empty());
        assert!(!text(&out).contains('\x1b'));
        assert!(text(&out).contains("--help"));
    }
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    assert_eq!(git(&f.repo, &["rev-parse", "origin/main"]), f.old);
    assert!(!f.repo.join(".git/repo-batch.lock").exists());
}

#[test]
fn clap_equals_short_flags_and_option_terminator_reach_the_public_runner() {
    let f = Fixture::new();
    let dash = f.clone("-directory");
    let out = f.run(
        &f.workspace,
        &[
            "--list",
            "-j2",
            "--attempts=2",
            "--timeout=30",
            "--remote=origin",
            "--branch=main",
            "-v",
            "--",
            "-directory",
        ],
    );
    success(&out);
    assert!(text(&out).contains("-directory"));
    assert_eq!(git(&dash, &["rev-parse", "HEAD"]), f.new);
    let out = f.run(
        &f.repo,
        &[
            "-j1",
            "--attempts=1",
            "--timeout=30",
            "--remote=origin",
            "--branch=main",
        ],
    );
    success(&out);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
}

#[test]
fn hidden_index_flags_refuse_before_fetch_and_preserve_local_edits() {
    for flag in ["--assume-unchanged", "--skip-worktree"] {
        for unchanged_remote in [true, false] {
            let f = Fixture::new();
            if unchanged_remote {
                git(&f.origin, &["update-ref", "refs/heads/main", &f.old]);
            }
            git(&f.repo, &["update-index", flag, "file"]);
            fs::write(f.repo.join("file"), "precious hidden edits").unwrap();
            assert!(git(
                &f.repo,
                &["status", "--porcelain=v1", "--untracked-files=no"]
            )
            .is_empty());
            let before_flags = git(&f.repo, &["ls-files", "-v"]);
            let out = f.run(&f.repo, &[]);
            failure(&out, "assume-unchanged or skip-worktree");
            assert_eq!(
                fs::read_to_string(f.repo.join("file")).unwrap(),
                "precious hidden edits"
            );
            assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
            assert_eq!(git(&f.repo, &["rev-parse", "origin/main"]), f.old);
            assert_eq!(git(&f.repo, &["ls-files", "-v"]), before_flags);
        }
    }
}

#[test]
fn checkout_hooks_cannot_redirect_reset_to_another_branch_or_detached_head() {
    for detached in [false, true] {
        let f = Fixture::new();
        git(&f.repo, &["branch", "victim"]);
        let hook = f.repo.join(".git/hooks/post-checkout");
        let args = if detached { "--detach HEAD" } else { "victim" };
        fs::write(
            &hook,
            format!("#!/bin/sh\ngit -c core.hooksPath=/dev/null switch {args}\n"),
        )
        .unwrap();
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
        let out = f.run(&f.repo, &[]);
        failure(&out, "Target branch changed");
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
        assert_eq!(git(&f.repo, &["rev-parse", "main"]), f.old);
        assert_eq!(git(&f.repo, &["rev-parse", "victim"]), f.old);
        assert_eq!(
            fs::read_to_string(f.repo.join("file")).unwrap(),
            "initial\n"
        );
        assert!(!fs::read_to_string(f.repo.join(".git/config"))
            .unwrap()
            .contains("[branch \"victim\"]"));
    }
}

#[test]
fn hook_logs_cannot_forge_worker_status() {
    let f = Fixture::new();
    let hook = f.repo.join(".git/hooks/post-checkout");
    fs::write(
        &hook,
        "#!/bin/sh\nprintf 'Target: spoofed\\033[31mvalue\\007\\nBackup: refs/home-reset-backups/nonexistent\\nStage: forged phase\\nState: forged state\\nAttempt 999/999\\n'\n",
    )
    .unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    let out = f.run(&f.repo, &[]);
    success(&out);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
    assert!(!text(&out).contains('\x1b'));
    assert!(!text(&out).contains('\x07'));
    assert!(!text(&out).contains("spoofed"));
    assert!(!text(&out).contains("nonexistent"));
    let results: serde_json::Value =
        serde_json::from_slice(&fs::read(logs(&out).join("results.json")).unwrap()).unwrap();
    let record = &results[0];
    assert_eq!(record["target"], format!("origin/main ({})", &f.new[..12]));
    assert_eq!(record["attempts"], 1);
    let backups = record["backups"].as_array().unwrap();
    assert_eq!(backups.len(), 1);
    assert_eq!(
        git(&f.repo, &["rev-parse", backups[0].as_str().unwrap()]),
        f.old
    );
}

#[test]
fn successful_git_diagnostics_cannot_suppress_a_transient_retry() {
    let f = Fixture::new();
    let count = f.temp.path().join("count");
    let dir = f.shim(&format!(
        r#"
for arg in "$@"; do
    if [ "$arg" = ls-files ]; then echo 'Permission denied: unrelated diagnostic' >&2; fi
    if [ "$arg" = fetch ]; then
        n=$(cat '{}' 2>/dev/null || echo 0); n=$((n+1)); echo $n > '{}'
        if [ "$n" -eq 1 ]; then echo 'Connection reset' >&2; exit 1; fi
    fi
done"#,
        count.display(),
        count.display()
    ));
    let mut cmd = f.cli(&f.repo, &[OsStr::new("--attempts"), OsStr::new("2")]);
    f.with_shim(&mut cmd, &dir);
    let out = cmd.output().unwrap();
    success(&out);
    assert_eq!(fs::read_to_string(count).unwrap().trim(), "2");
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
}

#[test]
fn worker_crash_reports_incomplete_status_without_fabricating_success() {
    let f = Fixture::new();
    let dir = f.shim(
        r#"for arg in "$@"; do
        if [ "$arg" = fetch ]; then kill -KILL "$PPID"; sleep 20; fi
    done"#,
    );
    let mut cmd = f.cli(&f.repo, &[]);
    f.with_shim(&mut cmd, &dir);
    let out = cmd.output().unwrap();
    failure(&out, "without a matching terminal status");
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    let results: serde_json::Value =
        serde_json::from_slice(&fs::read(logs(&out).join("results.json")).unwrap()).unwrap();
    assert_eq!(results[0]["attempts"], 1);
    assert_eq!(
        results[0]["target"],
        format!("origin/main ({})", &f.new[..12])
    );
}

#[test]
fn timeout_retries_retain_recovery_refs_from_each_attempt() {
    let f = Fixture::new();
    let count = f.temp.path().join("count");
    let dir = f.shim(&format!(
        r#"
for arg in "$@"; do
    if [ "$arg" = switch ]; then
        n=$(cat '{}' 2>/dev/null || echo 0); n=$((n+1)); echo $n > '{}'
        if [ "$n" -eq 1 ]; then sleep 20; fi
    fi
done"#,
        count.display(),
        count.display()
    ));
    // Leave room for real Git preflight under the 32-worker fixture load;
    // the injected 20-second switch hang still necessarily hits this deadline.
    let mut cmd = f.cli(
        &f.repo,
        &[OsStr::new("--attempts=2"), OsStr::new("--timeout=5")],
    );
    f.with_shim(&mut cmd, &dir);
    let out = cmd.output().unwrap();
    success(&out);
    let log_dir = logs(&out);
    let results: serde_json::Value =
        serde_json::from_slice(&fs::read(log_dir.join("results.json")).unwrap()).unwrap();
    assert_eq!(results[0]["attempts"], 2);
    let backups = results[0]["backups"].as_array().unwrap();
    assert_eq!(backups.len(), 2);
    for backup in backups {
        assert_eq!(
            git(&f.repo, &["rev-parse", backup.as_str().unwrap()]),
            f.old
        );
    }
    assert!(fs::read_to_string(log_dir.join("0001.log"))
        .unwrap()
        .contains("Timed out during switch"));
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
}

#[test]
fn reset_hooks_cannot_leave_dirty_files_behind_a_success_report() {
    let f = Fixture::new();
    let hook = f.repo.join(".git/hooks/post-index-change");
    fs::write(
        &hook,
        r#"#!/bin/sh
if [ "$1" = 1 ]; then
    if [ -f .git/reset-hook-armed ]; then
        printf 'precious reset-hook edits' > file
    else
        touch .git/reset-hook-armed
    fi
fi
"#,
    )
    .unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    let out = f.run(&f.repo, &[]);
    failure(&out, "post-reset verification");
    assert_eq!(
        fs::read_to_string(f.repo.join("file")).unwrap(),
        "precious reset-hook edits"
    );
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
    let records: serde_json::Value =
        serde_json::from_slice(&fs::read(logs(&out).join("results.json")).unwrap()).unwrap();
    assert_eq!(records[0]["code"], 1);
    assert_eq!(records[0]["attempts"], 1);
    let backups = records[0]["backups"].as_array().unwrap();
    assert_eq!(backups.len(), 1);
    assert_eq!(
        git(&f.repo, &["rev-parse", backups[0].as_str().unwrap()]),
        f.old
    );
    assert!(!text(&out).contains("1 succeeded"));
    assert!(String::from_utf8_lossy(&out.stdout).contains(&format!(
        "git branch recovered-work {}",
        backups[0].as_str().unwrap()
    )));
}

#[test]
fn safety_probes_do_not_trigger_index_refresh_hooks() {
    let f = Fixture::new();
    let hook = f.repo.join(".git/hooks/post-index-change");
    fs::write(
        &hook,
        r#"#!/bin/sh
if [ "$1" = 0 ]; then
    touch .git/refresh-hook-fired
    printf 'unexpected probe-hook edits' > file
fi
"#,
    )
    .unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    let out = f.run(&f.repo, &[]);
    success(&out);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
    assert_eq!(
        fs::read_to_string(f.repo.join("file")).unwrap(),
        "updated\n"
    );
    assert!(!f.repo.join(".git/refresh-hook-fired").exists());
}

#[test]
fn successful_default_runs_remove_logs_and_preserve_recovery_refs() {
    for (count, verbose) in [(1, false), (2, false), (1, true), (2, true)] {
        let f = Fixture::new();
        let mut repos = vec![f.repo.clone()];
        if count == 2 {
            repos.push(f.clone("second"));
        }
        // Every checkout needs a reset, so cleanup must preserve real recovery refs.
        for repo in &repos {
            git(repo, &["reset", "--hard", &f.old]);
        }
        let mut command = Command::new(BIN);
        env(command.current_dir(&f.workspace))
            .args(["git", "reset", "--attempts", "1"])
            .env("TMPDIR", f.temp.path().join("runtime"));
        if verbose {
            command.arg("--verbose");
        }
        let out = command.output().unwrap();
        success(&out);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.starts_with("Git checkout reset\n"));
        assert!(stdout.contains(&format!("Checkouts      {count}")));
        assert!(stdout.contains(&format!("Workers        {count}")));
        assert!(!text(&out).contains("Logs:"), "{}", text(&out));
        assert!(!text(&out).contains("in the logs"), "{}", text(&out));
        assert_eq!(
            fs::read_dir(f.temp.path().join("runtime")).unwrap().count(),
            0
        );
        assert_eq!(
            stdout.matches("  Recover ").count(),
            if verbose { count } else { 0 }
        );
        assert!(
            stdout.contains(&format!("{count} recovery refs saved")),
            "{stdout}"
        );
        if !verbose {
            assert!(stdout.contains("--verbose"));
        }
        for repo in &repos {
            if verbose {
                let reference = git(
                    repo,
                    &[
                        "for-each-ref",
                        "--format=%(refname)",
                        "refs/home-reset-backups/",
                    ],
                );
                assert!(stdout.contains(&format!("git branch recovered-work {reference}")));
            }
            assert_eq!(git(repo, &["rev-parse", "HEAD"]), f.new);
            assert_eq!(
                git(
                    repo,
                    &[
                        "for-each-ref",
                        "--format=%(objectname)",
                        "refs/home-reset-backups/"
                    ]
                ),
                f.old
            );
        }
    }
}

#[test]
fn failed_default_runs_retain_diagnostics_and_show_their_location() {
    let f = Fixture::new();
    fs::write(f.repo.join("file"), "local work\n").unwrap();
    let out = env(Command::new(BIN).current_dir(&f.repo))
        .args(["git", "reset", "--attempts", "1"])
        .env("TMPDIR", f.temp.path().join("runtime"))
        .output()
        .unwrap();
    failure(&out, "Tracked changes");
    assert!(!text(&out).contains("\n\n\n"), "{}", text(&out));
    let dir = logs(&out);
    assert!(dir.join("0001.log").is_file());
    assert!(dir.join("results.json").is_file());
    assert_eq!(
        fs::read_to_string(f.repo.join("file")).unwrap(),
        "local work\n"
    );
}

#[test]
fn optimized_default_pool_is_bounded_and_preserves_each_checkout() {
    let f = Fixture::new();
    let mut repos = vec![f.repo.clone()];
    for n in 1..33 {
        let repo = f.clone(&format!("repo-{n:02}"));
        git(&repo, &["reset", "--hard", &f.old]);
        repos.push(repo);
    }
    let trace = f.temp.path().join("default-worker-trace");
    let shim = f.shim(&format!(
        "for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then printf 'start\\n' >> '{}'; sleep 2; printf 'end\\n' >> '{}'; fi; done",
        trace.display(), trace.display()
    ));
    let mut cmd = Command::new(BIN);
    env(&mut cmd);
    cmd.args(["git", "reset", "--keep-logs", "--attempts=1"])
        .env("TMPDIR", f.temp.path().join("runtime"))
        .current_dir(&f.workspace);
    f.with_shim(&mut cmd, &shim);
    let out = cmd.output().unwrap();
    success(&out);
    let mut active = 0;
    let mut peak = 0;
    for event in fs::read_to_string(trace).unwrap().lines() {
        if event == "start" {
            active += 1;
        } else {
            active -= 1;
        }
        peak = peak.max(active);
        assert!(active <= 32);
    }
    assert_eq!(active, 0);
    assert!(
        peak > 4,
        "the default still serializes work into four slots"
    );
    assert!(text(&out).contains("Workers        32"));
    let results: serde_json::Value =
        serde_json::from_slice(&fs::read(logs(&out).join("results.json")).unwrap()).unwrap();
    assert_eq!(results.as_array().unwrap().len(), 33);
    for repo in repos {
        assert_eq!(git(&repo, &["rev-parse", "HEAD"]), f.new);
        assert_eq!(git(&repo, &["rev-parse", "@{upstream}"]), f.new);
        assert!(git(
            &repo,
            &[
                "--no-optional-locks",
                "status",
                "--porcelain",
                "--untracked-files=no"
            ]
        )
        .is_empty());
        assert_eq!(
            git(
                &repo,
                &[
                    "for-each-ref",
                    "--format=%(objectname)",
                    "refs/home-reset-backups/"
                ]
            ),
            f.old
        );
    }
}
