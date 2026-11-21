//! Acceptance tests exercise the public compiled CLI and real disposable Git repos.
mod common;
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
        .env("WORKCTL_STATE_DIR", common::state_dir())
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
    // Only the selected trunk was fetched; unrelated main is unchanged.
    assert_eq!(git(&f.repo, &["rev-parse", "origin/main"]), f.old);
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
fn stale_fsmonitor_cannot_hide_tracked_edits_from_reset() {
    let f = Fixture::new();
    let hook = f.temp.path().join("stale-fsmonitor");
    fs::write(&hook, "#!/bin/sh\nprintf 'token\\0'\n").unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    git(
        &f.repo,
        &["config", "core.fsmonitor", hook.to_str().unwrap()],
    );
    git(&f.repo, &["config", "core.fsmonitorHookVersion", "2"]);
    git(&f.repo, &["status", "--porcelain"]);
    git(&f.repo, &["update-index", "--fsmonitor-valid", "file"]);
    // Keep the fixture outside Git's independent racy-index fallback.
    fs::File::open(f.repo.join(".git/index"))
        .unwrap()
        .set_times(
            fs::FileTimes::new()
                .set_modified(std::time::SystemTime::now() + Duration::from_secs(3600)),
        )
        .unwrap();
    fs::write(f.repo.join("file"), "precious local edit\n").unwrap();
    let hidden = git(&f.repo, &["--no-optional-locks", "status", "--porcelain"]);
    assert!(
        hidden.is_empty(),
        "status {hidden:?}; flags {}",
        git(&f.repo, &["ls-files", "-f"])
    );

    let out = f.run(&f.repo, &[]);
    // Check preservation before exit status so the old implementation proves data loss.
    assert_eq!(
        fs::read_to_string(f.repo.join("file")).unwrap(),
        "precious local edit\n"
    );
    failure(&out, "Tracked changes");
    let detail = records(&out)[0]["detail"].as_str().unwrap().to_owned();
    let inspect = detail.split_once("; inspect with: ").unwrap().1;
    let inspected = env(Command::new("bash").current_dir(f.temp.path()))
        .args(["--noprofile", "--norc", "-c", inspect])
        .output()
        .unwrap();
    success(&inspected);
    assert!(String::from_utf8_lossy(&inspected.stdout).contains("M file"));
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    assert_eq!(
        git(&f.repo, &["config", "core.fsmonitor"]),
        hook.to_str().unwrap()
    );
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
        let descendant = f.temp.path().join("descendant");
        let release = f.temp.path().join("release-descendant");
        struct Release(PathBuf);
        impl Drop for Release {
            fn drop(&mut self) {
                let _ = fs::write(&self.0, "release");
            }
        }
        let _release = Release(release.clone());
        // Exercise a slow target lookup without expiring before the fetch fixture starts.
        let script = format!(
            "for arg in \"$@\"; do\n if [ \"$arg\" = ls-remote ]; then {} fi\n if [ \"$arg\" = fetch ]; then\n  (n=0; while [ ! -e '{}' ] && [ \"$n\" -lt 400 ]; do sleep 0.05; n=$((n+1)); done; [ ! -e '{}' ] || touch '{}') &\n  echo $! > '{}'\n  echo $$ > '{}'\n  {}\n fi\ndone",
            if mode == "timeout" { "sleep 2;" } else { ":;" },
            release.display(), release.display(), escaped.display(), descendant.display(),
            marker.display(), match mode {
                "normal" => ":",
                "timeout" => "while :; do printf 'Receiving objects from https://user:secret@example.com/repo\\r' >&2; sleep 1; done",
                _ => "sleep 20",
            }
        );
        let dir = f.shim(&script);
        let timeout = if mode == "timeout" { "8" } else { "30" };
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
        if mode == "timeout" {
            let log = fs::read_to_string(logs(&out).join("0001.log")).unwrap();
            assert!(log.contains("Last Git fetch output: Receiving objects from https://[redacted]@example.com/repo"), "{log}");
            assert!(!log.contains("user:secret"), "{log}");
        }
        let pid = fs::read_to_string(descendant)
            .unwrap()
            .trim()
            .parse::<i32>()
            .unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while unsafe { libc::kill(pid, 0) } == 0 {
            assert!(Instant::now() < until, "descendant survived {mode}");
            thread::sleep(Duration::from_millis(10));
        }
        fs::write(&release, "release").unwrap();
        assert!(!escaped.exists(), "descendant outlived {mode}");
    }
}

#[test]
fn detached_checkout_hook_cannot_keep_a_completed_reset_busy() {
    let f = Fixture::new();
    let ready = f.temp.path().join("hook-ready");
    let release = f.temp.path().join("hook-release");
    let done = f.temp.path().join("hook-done");
    struct ReleaseHook(PathBuf, PathBuf);
    impl Drop for ReleaseHook {
        fn drop(&mut self) {
            let _ = fs::write(&self.0, "release");
            let until = Instant::now() + Duration::from_secs(5);
            while !self.1.exists() && Instant::now() < until {
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
    let _cleanup = ReleaseHook(release.clone(), done.clone());
    let hook = f.repo.join(".git/hooks/post-checkout");
    fs::write(
        &hook,
        r#"#!/usr/bin/env python3
import os, time
ready = os.environ['PROBE_READY']
if not os.path.exists(ready):
    pid = os.fork()
    if pid == 0:
        os.setsid()
        devnull = os.open('/dev/null', os.O_RDWR)
        for fd in (0, 1, 2): os.dup2(devnull, fd)
        with open(ready, 'w') as f: f.write(str(os.getpid()))
        deadline = time.monotonic() + 30
        while not os.path.exists(os.environ['PROBE_RELEASE']) and time.monotonic() < deadline:
            time.sleep(0.01)
        os.closerange(3, 65536)
        with open(os.environ['PROBE_DONE'], 'w') as f: f.write('done')
        os._exit(0)
    deadline = time.monotonic() + 5
    while not os.path.exists(ready):
        if time.monotonic() > deadline: raise RuntimeError('hook did not start')
        time.sleep(0.01)
"#,
    )
    .unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    let run = || {
        f.cli(&f.repo, &[])
            .env("PROBE_READY", &ready)
            .env("PROBE_RELEASE", &release)
            .env("PROBE_DONE", &done)
            .output()
            .unwrap()
    };
    success(&run());
    wait_marker(&ready);
    let pid: i32 = fs::read_to_string(&ready).unwrap().parse().unwrap();
    assert_eq!(unsafe { libc::kill(pid, 0) }, 0, "hook must still be alive");
    assert!(!done.exists());
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
    // Repeat while the detached helper remains alive: success must release
    // coordination without unlinking the lock or terminating unrelated helpers.
    success(&run());
    assert_eq!(unsafe { libc::kill(pid, 0) }, 0);
    assert!(!done.exists());
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
}

#[test]
fn killed_supervisor_leaves_active_worker_holding_repository_lock() {
    use std::os::fd::AsRawFd;
    let f = Fixture::new();
    let marker = f.temp.path().join("fetch-started");
    let release = f.temp.path().join("fetch-release");
    let dir = f.shim(&format!(
        "for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then echo $$ > '{}'; n=0; while [ ! -e '{}' ] && [ \"$n\" -lt 600 ]; do sleep 0.05; n=$((n+1)); done; fi; done",
        marker.display(), release.display()
    ));
    struct ReleaseFetch(PathBuf);
    impl Drop for ReleaseFetch {
        fn drop(&mut self) {
            let _ = fs::write(&self.0, "release");
        }
    }
    let _cleanup = ReleaseFetch(release.clone());
    let mut cmd = f.cli(&f.repo, &[]);
    f.with_shim(&mut cmd, &dir);
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_marker(&marker);
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    // The worker must retain the supervisor's lock even though Git no longer
    // inherits it. A second reset must not mutate this repository concurrently.
    let blocked = f.run(&f.repo, &[]);
    failure(&blocked, "busy");
    let blocked_records = records(&blocked);
    let diagnostic = fs::read_to_string(blocked_records[0]["log"].as_str().unwrap()).unwrap();
    assert!(
        diagnostic.contains("Repository is busy"),
        "the advertised failure log must explain the failure: {diagnostic:?}"
    );
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    fs::write(&release, "release").unwrap();
    let lock = fs::File::open(f.repo.join(".git/repo-batch.lock")).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            break;
        }
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EWOULDBLOCK)
        );
        assert!(
            Instant::now() < until,
            "worker did not release coordination"
        );
        thread::sleep(Duration::from_millis(10));
    }
    drop(lock);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
    success(&f.run(&f.repo, &[]));
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
fn unavailable_remote_keeps_batch_failure_and_provides_scoped_recovery() {
    for json in [false, true] {
        let f = Fixture::new();
        let healthy = f.clone("healthy peer");
        let missing = f.temp.path().join("removed remote.git");
        git(
            &f.repo,
            &["remote", "set-url", "origin", origin_arg(&missing)],
        );
        for repo in [&f.repo, &healthy] {
            git(repo, &["remote", "rename", "origin", "upstream"]);
        }
        let mut args = vec![
            OsStr::new("--attempts=3"),
            OsStr::new("--remote=upstream"),
            OsStr::new("--branch=main"),
        ];
        if json {
            args.push(OsStr::new("--json"));
        }
        let out = f.cli(&f.workspace, &args).output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        let rows = if json {
            let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(result["summary"]["succeeded"], 1);
            assert_eq!(result["summary"]["failed"], 1);
            result["results"].clone()
        } else {
            serde_json::from_slice::<serde_json::Value>(
                &fs::read(logs(&out).join("results.json")).unwrap(),
            )
            .unwrap()
        };
        let bad = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["status"] == "failed")
            .unwrap();
        assert_eq!(bad["attempts"], 1);
        assert_eq!(bad["failure_kind"], "remote_unavailable");
        assert_eq!(bad["next_step"]["action"], "verify_remote_access");
        assert_eq!(bad["next_step"]["destructive"], false);
        assert!(bad["detail"]
            .as_str()
            .unwrap()
            .contains(origin_arg(&missing)));
        let check = bad["next_step"]["command"].as_str().unwrap();
        let retry = bad["next_step"]["retry_command"].as_str().unwrap();
        if !json {
            assert!(String::from_utf8_lossy(&out.stdout).contains(check));
            assert!(String::from_utf8_lossy(&out.stdout).contains(retry));
        }
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
        assert!(!f.repo.join(".git/FETCH_HEAD").exists());
        assert_eq!(git(&healthy, &["rev-parse", "HEAD"]), f.new);
        let checked = env(Command::new("bash").current_dir(f.temp.path()))
            .args(["-c", check])
            .output()
            .unwrap();
        assert!(!checked.status.success());
        assert!(text(&checked).contains(origin_arg(&missing)));
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
        git(
            &f.repo,
            &["remote", "set-url", "upstream", origin_arg(&f.origin)],
        );
        let bin = f.temp.path().join("retry-bin");
        fs::create_dir(&bin).unwrap();
        symlink(BIN, bin.join("workctl")).unwrap();
        let retried = env(Command::new("bash").current_dir(f.temp.path()))
            .args(["-c", retry])
            .env(
                "PATH",
                format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
            )
            .output()
            .unwrap();
        success(&retried);
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
        assert_eq!(git(&healthy, &["rev-parse", "HEAD"]), f.new);
    }
}

#[test]
fn remote_access_diagnostics_redact_credentials_at_resolution_and_fetch() {
    for stage in ["ls-remote", "fetch"] {
        let f = Fixture::new();
        let url = "https://test-user:private-password@bitbucket.org/workspace/repository.git?token=private-token";
        let count = f.temp.path().join("denials");
        let denial = if stage == "ls-remote" {
            format!("fatal: could not read Password for '{url}': terminal prompts disabled")
        } else {
            "You may not have access to this repository or it no longer exists in this workspace."
                .to_owned()
        };
        let script = format!(
            "for arg in \"$@\"; do\nif [ \"$arg\" = get-url ]; then echo '{url}'; exit 0; fi\nif [ \"$arg\" = {stage} ]; then echo denied >> '{}'; echo \"{denial}\" >&2; echo 'fatal: Could not read from remote repository.' >&2; exit 128; fi\ndone",
            count.display()
        );
        let dir = f.shim(&script);
        let mut cmd = f.cli(&f.repo, &[OsStr::new("--attempts=3"), OsStr::new("--json")]);
        f.with_shim(&mut cmd, &dir);
        let out = cmd.output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let row = &result["results"][0];
        assert_eq!(row["failure_kind"], "remote_unavailable");
        assert_eq!(row["next_step"]["action"], "verify_remote_access");
        assert_eq!(row["attempts"], 1);
        assert_eq!(fs::read_to_string(count).unwrap().lines().count(), 1);
        let log = fs::read_to_string(row["log"].as_str().unwrap()).unwrap();
        let diagnostics = format!("{}\n{log}", text(&out));
        assert!(diagnostics.contains("bitbucket.org/workspace/repository.git"));
        for secret in ["test-user", "private-password", "private-token"] {
            assert!(!diagnostics.contains(secret));
        }
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    }
}

#[test]
fn transient_failures_retry_but_authentication_failures_do_not() {
    for transient in [true, false] {
        let f = Fixture::new();
        let count = f.temp.path().join("count");
        let dir = f.shim(&format!("for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then n=$(cat '{}' 2>/dev/null || echo 0); n=$((n+1)); echo $n > '{}'; if [ $n -lt 2 ]; then echo '{}' >&2; echo 'fatal: Could not read from remote repository.' >&2; exit 1; fi; fi; done", count.display(), count.display(), if transient {"Connection reset"} else {"Permission denied (publickey)"}));
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
fn generic_disconnect_retries_fetch_but_not_initial_remote_lookup() {
    for stage in ["fetch", "ls-remote"] {
        let f = Fixture::new();
        let failed_once = f.temp.path().join("failed-once");
        let shim = f.shim(&format!(
            r#"for arg do
                if [ "$arg" = '{stage}' ] && [ ! -e '{}' ]; then
                    touch '{}'
                    printf '%s\n' 'fatal: Could not read from remote repository.' >&2
                    exit 128
                fi
            done"#,
            failed_once.display(),
            failed_once.display()
        ));
        let mut cmd = f.cli(&f.repo, &[OsStr::new("--attempts=2"), OsStr::new("--json")]);
        f.with_shim(&mut cmd, &shim);
        let out = cmd.output().unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        if stage == "fetch" {
            success(&out);
            assert_eq!(parsed["results"][0]["attempts"], 2);
            assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
        } else {
            assert_eq!(out.status.code(), Some(1));
            assert_eq!(parsed["results"][0]["attempts"], 1);
            assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
        }
    }
}

#[test]
fn shallow_fetch_uses_local_advertisement_without_changing_depth_or_target() {
    let f = Fixture::new();
    let shallow = f.workspace.join("shallow");
    git(&f.origin, &["update-ref", "refs/heads/main", &f.old]);
    let url = format!("file://{}", f.origin.display());
    git(
        &f.workspace,
        &["clone", "--depth=1", &url, origin_arg(&shallow)],
    );
    git(&f.origin, &["update-ref", "refs/heads/main", &f.new]);
    let shallow_path = shallow.join(".git/shallow");
    let boundary = fs::read(&shallow_path).unwrap();
    fs::write(
        shallow.join(".git/objects/info/alternates"),
        format!("{}/objects\n", f.origin.display()),
    )
    .unwrap();
    let marker = f.temp.path().join("hint");
    let shim = f.shim(&format!(
        r#"for arg do
            if [ "$arg" = '--negotiation-tip={}' ]; then
                touch '{}'
            fi
        done"#,
        f.new,
        marker.display()
    ));
    let mut cmd = f.cli(&shallow, &[]);
    f.with_shim(&mut cmd, &shim);
    success(&cmd.output().unwrap());
    assert!(
        marker.is_file(),
        "the locally available advertised commit was not offered"
    );
    assert_eq!(git(&shallow, &["rev-parse", "HEAD"]), f.new);
    assert_eq!(fs::read(&shallow_path).unwrap(), boundary);
}

#[test]
fn explicit_negotiation_tip_is_local_and_cannot_select_the_reset_commit() {
    let f = Fixture::new();
    let missing = "f".repeat(f.old.len());
    let absent = f.run(&f.repo, &["--negotiation-tip", &missing]);
    failure(&absent, "locally available commit");
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);

    let other = f.clone("another repo");
    let batch = f.run(&f.workspace, &["--negotiation-tip", &f.old]);
    failure(&batch, "requires exactly one selected checkout");
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    assert_eq!(git(&other, &["rev-parse", "HEAD"]), f.new);

    let marker = f.temp.path().join("explicit-tip");
    let shim = f.shim(&format!(
        "for arg do if [ \"$arg\" = '--negotiation-tip={}' ]; then touch '{}'; fi; done",
        f.old,
        marker.display()
    ));
    let mut cmd = f.cli(
        &f.repo,
        &[OsStr::new("--negotiation-tip"), OsStr::new(&f.old)],
    );
    f.with_shim(&mut cmd, &shim);
    success(&cmd.output().unwrap());
    assert!(marker.is_file());
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
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
    let done = f.temp.path().join("done");
    let release = f.temp.path().join("fetch-release");
    let dir = f.shim(&format!(
        r#"for arg do
            if [ "$arg" = fetch ]; then
                n=0
                while [ ! -e '{}' ] && [ "$n" -lt 1000 ]; do
                    sleep 0.05; n=$((n + 1))
                done
                touch '{}'
            fi
        done"#,
        release.display(),
        done.display()
    ));
    let output = f.temp.path().join("output");
    let mut cmd = f.cli(&f.workspace, &[]);
    f.with_shim(&mut cmd, &dir);
    let child = cmd
        .stdout(Stdio::piped())
        .stderr(fs::File::create(&output).unwrap())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(30);
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
    fs::write(&release, "release").unwrap();
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
fn reset_regression_unrelated_case_collisions_do_not_block_target_fetch() {
    let f = Fixture::new();
    git(&f.repo, &["fetch", "origin"]);
    for (name, oid) in [("NOISSUE/topic", &f.old), ("noissue/topic", &f.new)] {
        git(
            &f.origin,
            &["update-ref", &format!("refs/heads/{name}"), oid],
        );
        git(&f.origin, &["pack-refs", "--all"]);
        // Reproduce stale case-colliding tracking refs as well as remote branches.
        git(
            &f.repo,
            &["update-ref", &format!("refs/remotes/origin/{name}"), oid],
        );
        git(&f.repo, &["pack-refs", "--all"]);
    }
    git(&f.origin, &["update-ref", "refs/tags/unrelated", &f.new]);
    git(&f.repo, &["config", "remote.origin.prune", "true"]);
    git(&f.repo, &["config", "fetch.pruneTags", "true"]);
    git(&f.repo, &["config", "remote.origin.tagOpt", "--tags"]);
    let before = git(&f.repo, &["config", "--get-all", "remote.origin.fetch"]);
    let refs_before = git(
        &f.repo,
        &[
            "for-each-ref",
            "--format=%(refname) %(objectname)",
            "refs/remotes/origin/NOISSUE",
            "refs/remotes/origin/noissue",
        ],
    );
    success(&f.run(&f.repo, &[]));
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
    assert_eq!(
        git(&f.repo, &["config", "--get-all", "remote.origin.fetch"]),
        before
    );
    assert_eq!(
        git(
            &f.repo,
            &[
                "for-each-ref",
                "--format=%(refname) %(objectname)",
                "refs/remotes/origin/NOISSUE",
                "refs/remotes/origin/noissue"
            ]
        ),
        refs_before
    );
    assert!(git(&f.repo, &["tag", "--list"]).is_empty());
    for (name, oid) in [("NOISSUE/topic", &f.old), ("noissue/topic", &f.new)] {
        assert_eq!(
            git(&f.origin, &["rev-parse", &format!("refs/heads/{name}")]),
            *oid
        );
    }
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
fn reset_regression_remote_movement_uses_the_verified_fetched_tip() {
    for rewind in [false, true] {
        let f = Fixture::new();
        f.push_file("later");
        let latest = git(&f.seed, &["rev-parse", "HEAD"]);
        git(&f.origin, &["update-ref", "refs/heads/main", &f.new]);
        let fetched = if rewind { &f.old } else { &latest };
        let real = real_git();
        let dir = f.shim(&format!(
            "for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then '{}' --git-dir '{}' update-ref refs/heads/main '{}'; fi; done",
            real, f.origin.display(), fetched));
        let mut cmd = f.cli(&f.repo, &[]);
        f.with_shim(&mut cmd, &dir);
        let out = cmd.output().unwrap();
        success(&out);
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), *fetched);
        assert_eq!(git(&f.repo, &["rev-parse", "origin/main"]), *fetched);
        let records = records(&out);
        assert_eq!(records[0]["attempts"], 1);
        assert_eq!(
            records[0]["target"],
            format!("origin/main ({})", &fetched[..12])
        );
    }
}

#[test]
fn output_failure_after_workers_start_cancels_their_process_groups() {
    let f = Fixture::new();
    let dirty = f.clone("zz-dirty");
    fs::write(dirty.join("file"), "dirty").unwrap();
    let marker = f.temp.path().join("started");
    let escaped = f.temp.path().join("escaped");
    let release = f.temp.path().join("release-descendant");
    let stderr_closed = f.temp.path().join("stderr-closed");
    let dir=f.shim(&format!("case \"$PWD\" in *zz-dirty) case \"$*\" in *status*) while [ ! -e '{}' ]; do sleep 0.05; done; sleep 2;; esac;; esac\nfor arg in \"$@\"; do if [ \"$arg\" = fetch ]; then touch '{}'; (while [ ! -e '{}' ]; do sleep 0.05; done; touch '{}') & sleep 20; fi; done",stderr_closed.display(),marker.display(),release.display(),escaped.display()));
    let mut cmd = f.cli(&f.workspace, &[]);
    f.with_shim(&mut cmd, &dir);
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    struct Release(std::path::PathBuf);
    impl Drop for Release {
        fn drop(&mut self) {
            let _ = fs::write(&self.0, "closed\n");
        }
    }
    // A failed startup assertion must also release the waiting fixture process.
    let _release_on_failure = Release(stderr_closed.clone());
    wait_marker(&marker);
    drop(child.stderr.take());
    fs::write(stderr_closed, "closed\n").unwrap();
    let out = wait_child(child);
    assert_eq!(out.status.code(), Some(1));
    assert!(!escaped.exists());
    fs::write(release, "release\n").unwrap();
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
    // Close the read end before spawning, so even a fast child must see EPIPE.
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let child = f
        .cli(&f.repo, &[OsStr::new("--help")])
        .stdout(writer)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
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
fn late_timeout_verifies_applied_reset_without_following_a_new_remote_tip() {
    let f = Fixture::new();
    let count = f.temp.path().join("count");
    let dir = f.shim(&format!(
        r#"
for arg in "$@"; do
    if [ "$arg" = --hard ]; then
        if [ ! -e '{}' ]; then
            PATH="$FIXTURE_REAL_PATH" git "$@" || exit $?
            PATH="$FIXTURE_REAL_PATH" git -C '{}' update-ref refs/heads/main '{}' || exit $?
            touch '{}'
            sleep 45
            exit 0
        fi
    fi
done"#,
        count.display(),
        f.origin.display(),
        f.old,
        count.display()
    ));
    let mut cmd = f.cli(
        &f.repo,
        &[
            OsStr::new("--attempts=2"),
            OsStr::new("--timeout=15"),
            OsStr::new("--json"),
        ],
    );
    f.with_shim(&mut cmd, &dir);
    cmd.env("FIXTURE_REAL_PATH", std::env::var_os("PATH").unwrap());
    let out = cmd.output().unwrap();
    success(&out);
    let result = records(&out);
    assert_eq!(result[0]["attempts"], 1);
    assert!(result[0]["detail"]
        .as_str()
        .unwrap()
        .contains("verified after timeout"));
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
    assert_eq!(git(&f.origin, &["rev-parse", "HEAD"]), f.old);
    let backups = result[0]["backups"].as_array().unwrap();
    assert_eq!(backups.len(), 1);
    assert_eq!(
        git(&f.repo, &["rev-parse", backups[0].as_str().unwrap()]),
        f.old
    );
}

#[test]
fn late_timeout_preserves_local_edits_and_marks_the_checkout_unverified() {
    let f = Fixture::new();
    let dir = f.shim(
        r#"for arg do
            if [ "$arg" = --hard ]; then
                PATH="$FIXTURE_REAL_PATH" git "$@" || exit $?
                printf 'precious local edit' > file
                sleep 45
                exit 0
            fi
        done"#,
    );
    let mut cmd = f.cli(
        &f.repo,
        &[
            OsStr::new("--attempts=2"),
            OsStr::new("--timeout=15"),
            OsStr::new("--json"),
        ],
    );
    f.with_shim(&mut cmd, &dir);
    cmd.env("FIXTURE_REAL_PATH", std::env::var_os("PATH").unwrap());
    let out = cmd.output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["results"][0]["attempts"], 1);
    assert_eq!(result["results"][0]["failure_kind"], "checkout_unverified");
    assert_eq!(
        fs::read_to_string(f.repo.join("file")).unwrap(),
        "precious local edit"
    );
    let (_, history) = workctl_json(
        &f,
        &["history", result["run_id"].as_str().unwrap(), "--json"],
    );
    assert_eq!(history["targets"][0]["status"], "needs_verification");
}

fn workctl_json(f: &Fixture, args: &[&str]) -> (Output, serde_json::Value) {
    let out = env(&mut Command::new(BIN))
        .env("TMPDIR", f.temp.path().join("runtime"))
        .args(args)
        .output()
        .unwrap();
    let json = serde_json::from_slice(&out.stdout).unwrap();
    (out, json)
}

#[test]
fn recovery_keeps_a_completed_member_and_blocks_replay_of_the_running_member() {
    let f = Fixture::new();
    let second = f.clone("b-slow");
    git(&second, &["reset", "--hard", &f.old]);
    let marker = f.temp.path().join("slow-started");
    let release = f.temp.path().join("slow-release");
    let dir = f.shim(&format!(
        r#"if [ "${{PWD##*/}}" = b-slow ]; then
            for arg do
                if [ "$arg" = ls-remote ]; then
                    touch '{}'
                    n=0
                    while [ ! -e '{}' ] && [ "$n" -lt 120 ]; do
                        sleep 0.05; n=$((n + 1))
                    done
                    exit 1
                fi
            done
        fi"#,
        marker.display(),
        release.display()
    ));
    let mut cmd = f.cli(&f.workspace, &[OsStr::new("--jobs=1")]);
    f.with_shim(&mut cmd, &dir);
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_marker(&marker);
    let id = fs::read_dir(common::state_dir())
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let value: serde_json::Value =
                serde_json::from_slice(&fs::read(entry.path().join("summary.json")).ok()?).ok()?;
            value["targets"]
                .as_array()?
                .iter()
                .any(|target| {
                    target["path"] == second.canonicalize().unwrap().to_string_lossy().as_ref()
                })
                .then(|| value["run_id"].as_str().unwrap().to_owned())
        })
        .next()
        .unwrap();
    let (_, live) = workctl_json(&f, &["git", "reset", "--retry", &id, "--json"]);
    assert!(
        live["error"].as_str().unwrap().contains("still active"),
        "{live}"
    );
    child.kill().unwrap();
    child.wait().unwrap();
    fs::write(&release, "release").unwrap();

    let (_, history) = workctl_json(&f, &["history", &id, "--json"]);
    assert_eq!(history["targets"][0]["status"], "succeeded");
    assert_eq!(history["targets"][1]["status"], "running");
    let (out, retry) = workctl_json(&f, &["git", "reset", "--retry", &id, "--json"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(retry["retry_blocked"].as_array().unwrap().len(), 1);
    assert_eq!(git(&second, &["rev-parse", "HEAD"]), f.old);
}

#[test]
fn retry_continues_past_a_replaced_checkout_with_the_original_limits() {
    let f = Fixture::new();
    let second = f.clone("b-retry");
    let missing = f.temp.path().join("missing");
    for repo in [&f.repo, &second] {
        git(repo, &["remote", "set-url", "origin", origin_arg(&missing)]);
    }
    let failed = f.run(&f.workspace, &["--json", "--timeout=17", "--jobs=1"]);
    let first: serde_json::Value = serde_json::from_slice(&failed.stdout).unwrap();
    let id = first["run_id"].as_str().unwrap();
    fs::rename(&f.repo, f.temp.path().join("original")).unwrap();
    let replacement = f.clone("a repo");
    fs::write(replacement.join("untouched"), "local work").unwrap();
    git(
        &second,
        &["remote", "set-url", "origin", origin_arg(&f.origin)],
    );
    let (out, retry) = workctl_json(&f, &["git", "reset", "--retry", id, "--json"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(retry["summary"]["succeeded"], 1, "{retry}");
    assert_eq!(retry["summary"]["retry_blocked"], 1);
    assert_eq!(
        fs::read_to_string(replacement.join("untouched")).unwrap(),
        "local work"
    );
    let (_, history) = workctl_json(
        &f,
        &["history", retry["run_id"].as_str().unwrap(), "--json"],
    );
    assert_eq!(history["options"]["attempts"], 1);
    assert_eq!(history["options"]["timeout_seconds"], 17);
    assert_eq!(history["options"]["timeout_mode"], "fixed");
    assert_eq!(history["options"]["jobs"], 1);
}

#[test]
fn sparse_reset_preserves_exclusions_and_refuses_unexpected_masked_paths() {
    let f = Fixture::new();
    for directory in ["included", "omitted"] {
        fs::create_dir(f.seed.join(directory)).unwrap();
        fs::write(f.seed.join(directory).join("file"), "initial").unwrap();
    }
    git(&f.seed, &["add", "."]);
    git(&f.seed, &["commit", "-m", "add sparse paths"]);
    git(&f.seed, &["push", "origin", "main"]);
    git(&f.repo, &["pull", "--ff-only"]);
    git(
        &f.repo,
        &[
            "sparse-checkout",
            "set",
            "--cone",
            "--sparse-index",
            "included",
        ],
    );
    fs::write(f.seed.join("included/file"), "updated").unwrap();
    git(&f.seed, &["commit", "-am", "update included file"]);
    git(&f.seed, &["push", "origin", "main"]);

    success(&f.run(&f.repo, &[]));
    assert!(!f.repo.join("omitted/file").exists());
    assert_eq!(
        fs::read_to_string(f.repo.join("included/file")).unwrap(),
        "updated"
    );
    assert_eq!(
        git(&f.repo, &["ls-files", "-v", "omitted/file"]),
        "S omitted/file"
    );
    git(
        &f.repo,
        &["update-index", "--skip-worktree", "included/file"],
    );
    fs::remove_file(f.repo.join("included/file")).unwrap();
    failure(&f.run(&f.repo, &[]), "assume-unchanged or skip-worktree");
    assert!(!f.repo.join("included/file").exists());
}

#[test]
fn bitbucket_ssh_migration_retries_exact_remote_and_retains_git_configuration() {
    let f = Fixture::new();
    let rewritten = f.clone("b-https");
    let legacy = "git@bitbucket.org:workspace/repository.git";
    let https = "https://bitbucket.org/workspace/repository.git";
    git(&f.repo, &["remote", "set-url", "origin", legacy]);
    git(&rewritten, &["remote", "set-url", "origin", https]);
    git(
        &rewritten,
        &[
            "config",
            "url.git@bitbucket.org:.insteadOf",
            "https://bitbucket.org/",
        ],
    );
    let ssh = f.temp.path().join("ssh");
    let trace = f.temp.path().join("ssh-trace");
    fs::write(&ssh, r#"#!/bin/sh
    for arg do
        case "$arg" in
            git@bitbucket.org)
                echo old >> "$FIXTURE_TRACE"
                echo "We have moved our SSH host to ssh.bitbucket.org. The old host bitbucket.org doesn't accept SSH connections." >&2
                exit 1 ;;
            git@ssh.bitbucket.org)
                echo new >> "$FIXTURE_TRACE"
                exec git upload-pack "$FIXTURE_REMOTE" ;;
        esac
    done
    exit 99
    "#).unwrap();
    fs::set_permissions(&ssh, fs::Permissions::from_mode(0o755)).unwrap();
    let out = f
        .cli(
            &f.workspace,
            &[OsStr::new("--jobs=1"), OsStr::new("--json")],
        )
        .env("GIT_SSH_COMMAND", &ssh)
        .env("GIT_SSH_VARIANT", "ssh")
        .env("FIXTURE_REMOTE", &f.origin)
        .env("FIXTURE_TRACE", &trace)
        .output()
        .unwrap();
    success(&out);
    assert_eq!(
        fs::read_to_string(trace).unwrap(),
        "old\nnew\nnew\nold\nnew\nnew\n"
    );
    assert_eq!(git(&f.repo, &["config", "remote.origin.url"]), legacy);
    assert_eq!(git(&rewritten, &["config", "remote.origin.url"]), https);
    assert_eq!(git(&rewritten, &["rev-parse", "HEAD"]), f.new);
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
        "branch recovered-work {}",
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
            assert!(
                String::from_utf8_lossy(&out.stderr).ends_with("\n\n"),
                "{}",
                text(&out)
            );
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
                assert!(stdout.contains("git -C "));
                assert!(stdout.contains(&format!("branch recovered-work {reference}")));
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
fn partial_clone_checkout_retries_object_transfer_before_touching_the_checkout() {
    let f = Fixture::new();
    git(&f.origin, &["config", "uploadpack.allowFilter", "true"]);
    git(
        &f.origin,
        &["config", "uploadpack.allowAnySHA1InWant", "true"],
    );
    let partial = f.workspace.join("partial");
    let url = format!("file://{}", f.origin.display());
    git(
        &f.workspace,
        &["clone", "--filter=blob:none", &url, origin_arg(&partial)],
    );
    f.push_file("promised");
    let tip = git(&f.seed, &["rev-parse", "HEAD"]);
    let object = git(&f.seed, &["rev-parse", "HEAD:promised"]);
    let attempt = f.temp.path().join("hydration-attempt");
    let shim = f.shim(&format!(
        r#"
for arg in "$@"; do
    [ "$arg" = --stdin ] || continue
    case " $* " in *" fetch "*) ;; *) continue ;; esac
    if [ ! -f '{attempt}' ]; then
        test ! -e promised || exit 97
        test "$(git rev-parse HEAD)" = '{old}' || exit 97
        test -z "$(git for-each-ref refs/home-reset-backups)" || exit 97
        : > '{attempt}'
        echo 'Read from remote host ssh.bitbucket.org: Connection reset by peer' >&2
        echo 'fatal: early EOF' >&2
        exit 128
    fi
done
"#,
        attempt = attempt.display(),
        old = f.new,
    ));
    let run = |attempts| {
        let mut cmd = f.cli(
            &partial,
            &[
                OsStr::new("--json"),
                OsStr::new("--attempts"),
                OsStr::new(attempts),
            ],
        );
        f.with_shim(&mut cmd, &shim);
        cmd.env("GIT_NO_LAZY_FETCH", "1").output().unwrap()
    };
    let out = run("1");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let failed = &result["results"][0];
    assert_eq!(failed["stage"], "fetch checkout objects", "{failed}");
    assert_eq!(failed["failure_kind"], "fetch_disconnected", "{failed}");
    assert_eq!(failed["needs_verification"], false, "{failed}");
    assert_eq!(failed["backups"], serde_json::json!([]), "{failed}");
    assert_eq!(git(&partial, &["rev-parse", "HEAD"]), f.new);
    assert!(!partial.join("promised").exists());
    assert!(!env(Command::new("git").current_dir(&partial))
        .args(["--no-lazy-fetch", "cat-file", "-e", &object])
        .status()
        .unwrap()
        .success());

    fs::remove_file(&attempt).unwrap();
    let out = run("2");
    success(&out);
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["results"][0]["attempts"], 2);
    assert_eq!(git(&partial, &["rev-parse", "HEAD"]), tip);
    assert_eq!(git(&partial, &["rev-parse", "@{upstream}"]), tip);
    assert_eq!(
        fs::read_to_string(partial.join("promised")).unwrap(),
        "remote"
    );
    assert_eq!(
        git(&partial, &["--no-lazy-fetch", "cat-file", "-t", &object]),
        "blob"
    );
}

#[test]
fn reset_throttles_new_attempts_after_transfer_failures_until_the_batch_recovers() {
    let f = Fixture::new();
    let mut repos = vec![f.repo.clone()];
    for n in 1..6 {
        let repo = f.clone(&format!("repo-{n}"));
        git(&repo, &["reset", "--hard", &f.old]);
        repos.push(repo);
    }
    let trace = f.temp.path().join("trace");
    let first = f.temp.path().join("first");
    fs::create_dir(&first).unwrap();
    let shim = f.shim(&format!(
        r#"
for arg in "$@"; do
    [ "$arg" = fetch ] || continue
    marker="{first}/$(basename "$PWD")"
    if [ ! -f "$marker" ]; then
        : > "$marker"
        n=0
        while [ "$(find '{first}' -type f | wc -l)" -lt 6 ]; do
            n=$((n + 1))
            [ "$n" -le 400 ] || exit 97
            sleep 0.02
        done
        echo 'fatal: early EOF' >&2
        exit 128
    fi
    if [ ! -f "$marker.second" ]; then
        : > "$marker.second"
        printf 'second-start\n' >> '{trace}'
        sleep 1
        printf 'second-end\n' >> '{trace}'
        echo 'fatal: early EOF' >&2
        exit 128
    fi
    printf 'third-start\n' >> '{trace}'
    sleep 0.1
    printf 'third-end\n' >> '{trace}'
    break
done
"#,
        first = first.display(),
        trace = trace.display(),
    ));
    let mut cmd = f.cli(
        &f.workspace,
        &[
            OsStr::new("--jobs=6"),
            OsStr::new("--attempts=3"),
            OsStr::new("--json"),
        ],
    );
    f.with_shim(&mut cmd, &shim);
    let out = cmd.output().unwrap();
    success(&out);
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(result["results"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["attempts"] == 3));
    let mut active_second = 0;
    let mut active_third = 0;
    let mut total_third = 0;
    for event in fs::read_to_string(trace).unwrap().lines() {
        match event {
            "second-start" => active_second += 1,
            "second-end" => active_second -= 1,
            "third-start" => {
                active_third += 1;
                total_third += 1;
            }
            "third-end" => active_third -= 1,
            _ => panic!("unexpected trace event {event}"),
        }
        assert!(
            active_second <= 4 && active_third <= 1,
            "fetch concurrency exceeded recovery limits"
        );
    }
    assert_eq!((active_second, active_third, total_third), (0, 0, 6));
    for repo in repos {
        assert_eq!(git(&repo, &["rev-parse", "HEAD"]), f.new);
    }
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

fn real_git() -> String {
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

fn records(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&fs::read(logs(out).join("results.json")).unwrap()).unwrap()
}

#[test]
fn reset_regression_shared_untracked_directories_preserve_local_contents() {
    for kind in ["empty", "untracked", "ignored", "nested"] {
        let f = Fixture::new();
        fs::create_dir(f.seed.join(".claude")).unwrap();
        f.push_file(".claude/settings.json");
        let local = f.repo.join(".claude/.cc-writes");
        fs::create_dir_all(&local).unwrap();
        if kind != "empty" {
            fs::write(local.join("precious"), "local contents").unwrap();
        }
        if kind == "ignored" {
            fs::write(f.repo.join(".git/info/exclude"), ".claude/\n").unwrap();
        }
        if kind == "nested" {
            git(&local, &["init", "-b", "main"]);
            git(&local, &["add", "."]);
            git(&local, &["commit", "-m", "nested work"]);
        }
        let out = f.run(&f.repo, &[]);
        success(&out);
        assert!(local.is_dir(), "{kind}");
        if kind != "empty" {
            assert_eq!(
                fs::read_to_string(local.join("precious")).unwrap(),
                "local contents"
            );
        }
        if kind == "nested" {
            assert_eq!(git(&local, &["log", "-1", "--format=%s"]), "nested work");
        }
        assert_eq!(
            fs::read_to_string(f.repo.join(".claude/settings.json")).unwrap(),
            "remote"
        );
        assert_eq!(
            git(&f.repo, &["rev-parse", "HEAD"]),
            git(&f.seed, &["rev-parse", "HEAD"])
        );
    }
}

#[test]
fn reset_regression_shared_directories_still_protect_real_collisions() {
    for kind in ["file", "ignored", "symlink", "nested"] {
        let f = Fixture::new();
        fs::create_dir(f.seed.join(".claude")).unwrap();
        f.push_file(".claude/settings.json");
        let local = f.repo.join(".claude");
        fs::create_dir(&local).unwrap();
        fs::write(local.join("settings.json"), "precious").unwrap();
        if kind == "ignored" {
            fs::write(f.repo.join(".git/info/exclude"), ".claude/\n").unwrap();
        }
        if kind == "symlink" {
            let outside = f.temp.path().join("outside");
            fs::rename(&local, &outside).unwrap();
            symlink(outside, &local).unwrap();
        }
        if kind == "nested" {
            git(&local, &["init", "-b", "main"]);
            git(&local, &["add", "."]);
            git(&local, &["commit", "-m", "nested work"]);
        }
        let out = f.run(&f.repo, &[]);
        failure(&out, "would be overwritten");
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
        assert_eq!(
            fs::read_to_string(local.join("settings.json")).unwrap(),
            "precious"
        );
    }
}

#[test]
fn reset_regression_stale_or_mismatched_fetch_evidence_cannot_reset() {
    for kind in ["stale", "wrong-branch", "wrong-tip"] {
        let f = Fixture::new();
        git(&f.repo, &["fetch", "origin"]);
        let real = real_git();
        let action = match kind {
            "stale" => "exit 0".to_owned(),
            "wrong-branch" => format!("'{}' \"$@\" || exit $?; printf '{}\\t\\tbranch '\\''other'\\'' of nowhere\\n' > .git/FETCH_HEAD; exit 0", real, f.new),
            _ => format!("'{}' \"$@\" || exit $?; '{}' update-ref refs/remotes/origin/main '{}'; exit 0", real, real, f.old),
        };
        let dir = f.shim(&format!(
            "for arg in \"$@\"; do if [ \"$arg\" = fetch ]; then {action}; fi; done"
        ));
        let mut cmd = f.cli(&f.repo, &[]);
        f.with_shim(&mut cmd, &dir);
        let out = cmd.output().unwrap();
        failure(&out, "fetch");
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
        assert!(records(&out)[0]["backups"].as_array().unwrap().is_empty());
    }
}

#[test]
fn reset_regression_symbolic_fetch_destination_cannot_replace_local_branch() {
    let f = Fixture::new();
    git(&f.repo, &["branch", "precious"]);
    git(
        &f.repo,
        &[
            "symbolic-ref",
            "refs/remotes/origin/main",
            "refs/heads/precious",
        ],
    );
    let out = f.run(&f.repo, &[]);
    failure(&out, "symbolic");
    assert_eq!(git(&f.repo, &["rev-parse", "precious"]), f.old);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
}

#[test]
fn reset_regression_selected_case_alias_is_refused_without_ref_changes() {
    let f = Fixture::new();
    let probe = f.repo.join(".git/case-probe");
    fs::write(&probe, "").unwrap();
    let insensitive = f.repo.join(".git/CASE-PROBE").exists();
    fs::remove_file(probe).unwrap();
    git(&f.repo, &["update-ref", "-d", "refs/remotes/origin/main"]);
    git(&f.repo, &["update-ref", "refs/remotes/origin/MAIN", &f.old]);
    let before = git(
        &f.repo,
        &["for-each-ref", "--format=%(refname) %(objectname)"],
    );
    let out = f.run(&f.repo, &[]);
    if insensitive {
        failure(&out, "case");
        assert_eq!(
            git(
                &f.repo,
                &["for-each-ref", "--format=%(refname) %(objectname)"]
            ),
            before
        );
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    } else {
        success(&out);
        assert_eq!(
            git(&f.repo, &["rev-parse", "refs/remotes/origin/MAIN"]),
            f.old
        );
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
    }
}

#[test]
fn successful_fetch_stdout_redacts_url_credentials_before_logging() {
    let f = Fixture::new();
    let dir = f.shim(
        r#"
for arg in "$@"; do
    if [ "$arg" = fetch ]; then
        printf 'fetch diagnostic: https://user:FETCH_PASSWORD@example.com/repo?token=FETCH_TOKEN\n'
    fi
done"#,
    );
    let mut cmd = f.cli(&f.repo, &[OsStr::new("--verbose")]);
    f.with_shim(&mut cmd, &dir);
    let out = cmd.output().unwrap();
    success(&out);
    let log = fs::read_to_string(logs(&out).join("0001.log")).unwrap();
    for output in [text(&out), log] {
        assert!(
            !output.contains("FETCH_PASSWORD"),
            "credential leaked into diagnostic output"
        );
        assert!(
            !output.contains("FETCH_TOKEN"),
            "token leaked into diagnostic output"
        );
        assert!(output
            .contains("fetch diagnostic: https://[redacted]@example.com/repo?token=[redacted]"));
    }
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
}

#[test]
fn reset_regression_errors_are_bounded_readable_and_keep_full_diagnostics() {
    let f = Fixture::new();
    let dir = f.shim(r#"
for arg in "$@"; do
    if [ "$arg" = fetch ]; then
        printf 'error: cannot lock ref\nlock diagnostic\033[31m\n' >&2
        n=0; while [ "$n" -lt 100 ]; do printf ' - [deleted] (none) -> origin/noisy-branch-%s\n' "$n" >&2; n=$((n+1)); done
        echo 'END-OF-DIAGNOSTIC' >&2
        exit 1
    fi
done"#);
    let mut cmd = f.cli(&f.repo, &[]);
    f.with_shim(&mut cmd, &dir);
    let out = cmd.output().unwrap();
    failure(&out, "cannot lock ref");
    let rendered = text(&out);
    assert!(!rendered.contains("\\u{a}"), "{rendered}");
    assert!(!rendered.contains('\x1b'), "{rendered}");
    assert!(
        rendered.len() < 2200,
        "unbounded error output: {}",
        rendered.len()
    );
    assert!(!rendered.contains("END-OF-DIAGNOSTIC"));
    let log = fs::read_to_string(logs(&out).join("0001.log")).unwrap();
    assert!(log.contains("END-OF-DIAGNOSTIC"));
    assert!(records(&out)[0]["detail"]
        .as_str()
        .unwrap()
        .contains("END-OF-DIAGNOSTIC"));
}

#[test]
fn reset_regression_network_exhaustion_is_bounded_and_reports_attempts() {
    for stage in ["ls-remote", "fetch"] {
        let f = Fixture::new();
        let count = f.temp.path().join("count");
        let dir = f.shim(&format!(
            "for arg in \"$@\"; do if [ \"$arg\" = '{stage}' ]; then echo attempt >> '{}'; echo 'Failed to connect to github.com port 443' >&2; exit 128; fi; done",
            count.display()));
        let mut cmd = f.cli(&f.repo, &[OsStr::new("--attempts=2")]);
        f.with_shim(&mut cmd, &dir);
        let out = cmd.output().unwrap();
        failure(&out, "Failed to connect");
        assert_eq!(fs::read_to_string(count).unwrap().lines().count(), 2);
        assert_eq!(records(&out)[0]["attempts"], 2);
        assert!(text(&out).contains("2 attempts"), "{}", text(&out));
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
        assert!(records(&out)[0]["backups"].as_array().unwrap().is_empty());
    }
}

#[test]
fn reset_regression_shared_directory_pathspecs_preserve_literal_non_utf8_names() {
    let f = Fixture::new();
    let name = OsString::from_vec(if cfg!(target_os = "linux") {
        b"[literal]*-\n\xff".to_vec()
    } else {
        "[literal]*-\né".as_bytes().to_vec()
    });
    fs::create_dir(f.seed.join(&name)).unwrap();
    fs::write(f.seed.join(&name).join("remote"), "remote").unwrap();
    git(&f.seed, &["add", "."]);
    git(&f.seed, &["commit", "-m", "literal directory"]);
    git(&f.seed, &["push", "origin", "main"]);
    fs::create_dir(f.repo.join(&name)).unwrap();
    fs::write(f.repo.join(&name).join("local"), "precious").unwrap();
    success(&f.run(&f.repo, &[]));
    assert_eq!(
        fs::read(f.repo.join(&name).join("local")).unwrap(),
        b"precious"
    );
    assert_eq!(
        fs::read(f.repo.join(&name).join("remote")).unwrap(),
        b"remote"
    );
}

#[test]
fn reset_regression_selected_local_branch_case_alias_is_preserved() {
    let f = Fixture::new();
    let probe = f.repo.join(".git/case-probe");
    fs::write(&probe, "").unwrap();
    let insensitive = f.repo.join(".git/CASE-PROBE").exists();
    fs::remove_file(probe).unwrap();
    git(&f.repo, &["branch", "-m", "temporary"]);
    git(&f.repo, &["branch", "-m", "MAIN"]);
    let out = f.run(&f.repo, &[]);
    if insensitive {
        failure(&out, "case");
        assert_eq!(git(&f.repo, &["branch", "--show-current"]), "MAIN");
        assert_eq!(git(&f.repo, &["rev-parse", "origin/main"]), f.old);
    } else {
        success(&out);
        assert_eq!(git(&f.repo, &["branch", "--show-current"]), "main");
    }
    assert_eq!(git(&f.repo, &["rev-parse", "MAIN"]), f.old);
}

#[test]
fn reset_regression_explicit_branch_names_are_verified_literally() {
    let f = Fixture::new();
    for branch in ["feature/topic", "feature/o'hare", "feature/café"] {
        git(
            &f.origin,
            &["update-ref", &format!("refs/heads/{branch}"), &f.new],
        );
        success(&f.run(&f.repo, &["--branch", branch]));
        assert_eq!(git(&f.repo, &["branch", "--show-current"]), branch);
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
    }
}

#[test]
fn reset_recovery_commands_quote_refs_and_select_the_correct_repository() {
    for branch in [
        "feature/$(touch${IFS}unexpected-recovery)",
        "feature/o'hare",
    ] {
        let f = Fixture::new();
        git(&f.repo, &["branch", branch, &f.old]);
        git(
            &f.origin,
            &["update-ref", &format!("refs/heads/{branch}"), &f.new],
        );
        let out = f.run(&f.repo, &["--branch", branch, "--verbose"]);
        success(&out);
        let stdout = String::from_utf8(out.stdout.clone()).unwrap();
        let recovery = stdout
            .lines()
            .find_map(|line| {
                line.strip_prefix("  Recover ")
                    .and_then(|line| line.split_once(": "))
                    .map(|(_, command)| command)
            })
            .expect("reset must supply a recovery command");
        let recovered = env(Command::new("bash").current_dir(f.temp.path()))
            .args(["--noprofile", "--norc", "-c", recovery])
            .output()
            .unwrap();
        assert!(
            !f.temp.path().join("unexpected-recovery").exists(),
            "recovery command executed branch text"
        );
        success(&recovered);
        assert_eq!(git(&f.repo, &["rev-parse", "recovered-work"]), f.old);
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.new);
        git(&f.repo, &["branch", "-D", "recovered-work"]);
        let log = fs::read_to_string(logs(&out).join("0001.log")).unwrap();
        let recovery = log
            .lines()
            .find_map(|line| line.strip_prefix("Recover with: "))
            .unwrap();
        let recovered = env(Command::new("bash").current_dir(f.temp.path()))
            .args(["--noprofile", "--norc", "-c", recovery])
            .output()
            .unwrap();
        success(&recovered);
        assert!(!f.temp.path().join("unexpected-recovery").exists());
        assert_eq!(git(&f.repo, &["rev-parse", "recovered-work"]), f.old);
    }
}

#[test]
fn reset_regression_shared_directories_protect_case_aliases() {
    for ignored in [false, true] {
        let f = Fixture::new();
        fs::create_dir(f.seed.join(".claude")).unwrap();
        f.push_file(".claude/settings.json");
        fs::create_dir(f.repo.join(".claude")).unwrap();
        fs::write(f.repo.join(".claude/SETTINGS.json"), "precious").unwrap();
        let insensitive = f.repo.join(".claude/settings.json").exists();
        if ignored {
            fs::write(f.repo.join(".git/info/exclude"), ".claude/\n").unwrap();
        }
        // Probe the filesystem itself, not a potentially stale Git setting.
        git(&f.repo, &["config", "core.ignorecase", "false"]);
        let out = f.run(&f.repo, &[]);
        if insensitive {
            failure(&out, "would be overwritten");
            assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
        } else {
            success(&out);
            assert_eq!(
                fs::read_to_string(f.repo.join(".claude/settings.json")).unwrap(),
                "remote"
            );
        }
        assert_eq!(
            fs::read_to_string(f.repo.join(".claude/SETTINGS.json")).unwrap(),
            "precious"
        );
    }
}

#[test]
fn excluding_a_descendant_protects_the_entire_repository_from_reset() {
    let f = Fixture::new();
    let out = f
        .cli(
            &f.workspace,
            &[OsStr::new("--exclude"), f.repo.join("file").as_os_str()],
        )
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), f.old);
    assert_eq!(
        fs::read_to_string(f.repo.join("file")).unwrap(),
        "initial\n"
    );
}
