//! Direct-to-main publishing contracts against real disposable Git repositories.
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_config-tools");
const ABSENT: &str = "0000000000000000000000000000000000000000";

fn env(command: &mut Command) -> &mut Command {
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .env_remove("CONFIG_TOOLS_PUBLISH");
    for name in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_CONFIG_COUNT",
    ] {
        command.env_remove(name);
    }
    command
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = env(Command::new("git").current_dir(dir))
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {}", text(&output));
    String::from_utf8(output.stdout).unwrap().trim().into()
}

/// A bare `origin`, its canonical checkout on `main`, and a detached task worktree.
struct Fixture {
    temp: tempfile::TempDir,
    origin: PathBuf,
    canonical: PathBuf,
    task: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (origin, canonical, task) = (
            root.join("origin.git"),
            root.join("canonical"),
            root.join("task"),
        );
        git(
            &root,
            &["init", "--quiet", "--bare", "-b", "main", "origin.git"],
        );
        git(&root, &["init", "--quiet", "-b", "main", "canonical"]);
        fs::write(canonical.join("file"), "published\n").unwrap();
        git(&canonical, &["add", "."]);
        git(&canonical, &["commit", "--quiet", "-m", "initial"]);
        git(
            &canonical,
            &["remote", "add", "origin", origin.to_str().unwrap()],
        );
        git(&canonical, &["push", "--quiet", "-u", "origin", "main"]);
        git(
            &canonical,
            &[
                "worktree",
                "add",
                "--quiet",
                "--detach",
                task.to_str().unwrap(),
            ],
        );
        Self {
            temp,
            origin,
            canonical,
            task,
        }
    }

    /// Commits one change in `dir` and returns the new commit.
    fn commit(&self, dir: &Path, name: &str) -> String {
        fs::write(dir.join(name), name).unwrap();
        git(dir, &["add", name]);
        git(dir, &["commit", "--quiet", "-m", name]);
        git(dir, &["rev-parse", "HEAD"])
    }

    /// Lands a commit on `origin/main` from an unrelated clone, as a concurrent session would.
    fn concurrent_push(&self, name: &str) -> String {
        let clone = self.temp.path().join(format!("clone-{name}"));
        git(
            self.temp.path(),
            &[
                "clone",
                "--quiet",
                self.origin.to_str().unwrap(),
                clone.to_str().unwrap(),
            ],
        );
        let commit = self.commit(&clone, name);
        git(&clone, &["push", "--quiet", "origin", "main"]);
        commit
    }

    fn publish(&self, dir: &Path) -> Output {
        env(Command::new(BIN).arg("publish").current_dir(dir))
            .output()
            .unwrap()
    }

    fn origin_tip(&self) -> String {
        git(&self.origin, &["rev-parse", "refs/heads/main"])
    }

    fn canonical_head(&self) -> String {
        git(&self.canonical, &["rev-parse", "HEAD"])
    }
}

#[test]
fn publish_pushes_a_fast_forward_and_synchronizes_the_canonical_checkout() {
    let f = Fixture::new();
    let commit = f.commit(&f.task, "change");
    let output = f.publish(&f.task);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output));
    assert_eq!(f.origin_tip(), commit);
    assert_eq!(f.canonical_head(), commit);
    assert_eq!(git(&f.canonical, &["status", "--porcelain"]), "");
    assert_eq!(
        git(&f.canonical, &["symbolic-ref", "--short", "HEAD"]),
        "main"
    );
    assert_eq!(
        fs::read_to_string(f.canonical.join("change")).unwrap(),
        "change"
    );
    assert!(
        text(&output).contains("remote tip verified"),
        "{}",
        text(&output)
    );

    // Publishing from the canonical checkout itself needs no second step.
    let direct = f.commit(&f.canonical, "direct");
    let output = f.publish(&f.canonical);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output));
    assert_eq!(f.origin_tip(), direct);
}

#[test]
fn publish_never_pushes_over_commits_it_does_not_contain() {
    let f = Fixture::new();
    let theirs = f.concurrent_push("theirs");
    let before = f.canonical_head();
    f.commit(&f.task, "mine");
    let output = f.publish(&f.task);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output));
    assert!(
        text(&output).contains("Integrate it here"),
        "{}",
        text(&output)
    );
    assert_eq!(f.origin_tip(), theirs);
    assert_eq!(f.canonical_head(), before);

    // Integrating as instructed makes the same publish succeed.
    git(&f.task, &["merge", "--quiet", "--no-edit", "origin/main"]);
    let merged = git(&f.task, &["rev-parse", "HEAD"]);
    let output = f.publish(&f.task);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output));
    assert_eq!(f.origin_tip(), merged);
    assert_eq!(f.canonical_head(), merged);
}

#[test]
fn publish_refuses_to_leave_uncommitted_or_unfinished_work_behind() {
    let f = Fixture::new();
    let before = f.origin_tip();
    f.commit(&f.task, "change");
    fs::write(f.task.join("file"), "edited but not committed\n").unwrap();
    let output = f.publish(&f.task);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output));
    assert!(
        text(&output).contains("uncommitted tracked changes"),
        "{}",
        text(&output)
    );
    assert_eq!(f.origin_tip(), before);

    git(&f.task, &["checkout", "--quiet", "--", "file"]);
    let other = f.concurrent_push("file");
    git(&f.task, &["fetch", "--quiet", "origin"]);
    fs::write(f.task.join("file"), "conflicting\n").unwrap();
    git(&f.task, &["commit", "--quiet", "-am", "conflict"]);
    assert!(!env(Command::new("git").current_dir(&f.task))
        .args(["merge", "origin/main"])
        .output()
        .unwrap()
        .status
        .success());
    let output = f.publish(&f.task);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output));
    assert!(
        text(&output).contains("merge is in progress"),
        "{}",
        text(&output)
    );
    assert_eq!(f.origin_tip(), other);
}

#[test]
fn publish_reports_a_canonical_checkout_it_must_not_touch() {
    for state in ["tracked change", "local commit", "other branch"] {
        let f = Fixture::new();
        match state {
            "tracked change" => fs::write(f.canonical.join("file"), "someone's work\n").unwrap(),
            "local commit" => {
                f.commit(&f.canonical, "unpublished");
            }
            _ => {
                git(&f.canonical, &["switch", "--quiet", "-c", "side"]);
            }
        }
        let before = f.canonical_head();
        let commit = f.commit(&f.task, "change");
        let output = f.publish(&f.task);
        assert_eq!(output.status.code(), Some(3), "{state}: {}", text(&output));
        assert!(
            text(&output).contains("NOT synchronized"),
            "{state}: {}",
            text(&output)
        );
        assert_eq!(f.origin_tip(), commit, "{state}");
        assert_eq!(f.canonical_head(), before, "{state}");
        if state == "tracked change" {
            assert_eq!(
                fs::read_to_string(f.canonical.join("file")).unwrap(),
                "someone's work\n"
            );
        }
    }
}

#[test]
fn publish_with_nothing_new_still_brings_the_canonical_checkout_up_to_date() {
    let f = Fixture::new();
    let theirs = f.concurrent_push("theirs");
    let output = f.publish(&f.task);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output));
    assert!(
        text(&output).contains("Nothing to publish"),
        "{}",
        text(&output)
    );
    assert_eq!(f.origin_tip(), theirs);
    assert_eq!(f.canonical_head(), theirs);
}

fn pre_push(dir: &Path, remote: &str, url: &str, updates: &str, marked: bool) -> Output {
    use std::io::Write;
    let mut command = Command::new(BIN);
    env(command.args(["pre-push", remote, url]).current_dir(dir))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if marked {
        command.env("CONFIG_TOOLS_PUBLISH", "1");
    }
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(updates.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn pre_push_allows_only_a_published_fast_forward_of_main() {
    let f = Fixture::new();
    let base = f.origin_tip();
    let next = f.commit(&f.task, "change");
    let unrelated = {
        git(&f.task, &["switch", "--quiet", "--orphan", "unrelated"]);
        git(
            &f.task,
            &["commit", "--quiet", "--allow-empty", "-m", "unrelated"],
        );
        git(&f.task, &["rev-parse", "HEAD"])
    };
    let origin = f.origin.to_str().unwrap();
    let main = |local: &str, remote: &str| format!("HEAD {local} refs/heads/main {remote}\n");
    let cases: [(&str, String, bool, Option<&str>); 7] = [
        ("published fast-forward", main(&next, &base), true, None),
        (
            "bare push",
            main(&next, &base),
            false,
            Some("cargo run -- publish"),
        ),
        (
            "branch",
            format!("HEAD {next} refs/heads/feature {ABSENT}\n"),
            true,
            Some("refusing to push refs/heads/feature"),
        ),
        (
            "tag",
            format!("HEAD {next} refs/tags/v1 {ABSENT}\n"),
            true,
            Some("refusing to push refs/tags/v1"),
        ),
        (
            "delete",
            main(ABSENT, &base),
            true,
            Some("refusing to delete"),
        ),
        (
            "rewrite",
            main(&unrelated, &base),
            true,
            Some("not a fast-forward"),
        ),
        ("nothing to push", String::new(), false, None),
    ];
    for (name, updates, marked, refusal) in cases {
        let output = pre_push(&f.task, "origin", origin, &updates, marked);
        match refusal {
            None => assert!(output.status.success(), "{name}: {}", text(&output)),
            Some(reason) => {
                assert_eq!(output.status.code(), Some(1), "{name}: {}", text(&output));
                assert!(text(&output).contains(reason), "{name}: {}", text(&output));
            }
        }
    }

    // The same remote reached by URL is held to the same rule; other remotes are not.
    let by_url = pre_push(&f.task, origin, origin, &main(&next, &base), false);
    assert_eq!(by_url.status.code(), Some(1), "{}", text(&by_url));
    let elsewhere = pre_push(
        &f.task,
        "fork",
        "/somewhere/else.git",
        &format!("HEAD {next} refs/heads/feature {ABSENT}\n"),
        false,
    );
    assert!(elsewhere.status.success(), "{}", text(&elsewhere));
}

#[test]
fn installed_hook_rejects_a_bare_push_and_accepts_publish() {
    let f = Fixture::new();
    let hooks = f.temp.path().join("hooks");
    fs::create_dir(&hooks).unwrap();
    let hook = hooks.join("pre-push");
    fs::write(&hook, format!("#!/bin/sh\nexec '{BIN}' pre-push \"$@\"\n")).unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    git(
        &f.canonical,
        &["config", "core.hooksPath", hooks.to_str().unwrap()],
    );
    let before = f.origin_tip();
    let commit = f.commit(&f.task, "change");

    for refspec in ["HEAD:refs/heads/main", "HEAD:refs/heads/review-branch"] {
        let bare = env(Command::new("git").current_dir(&f.task))
            .args(["push", "origin", refspec])
            .output()
            .unwrap();
        assert!(!bare.status.success(), "{refspec}: {}", text(&bare));
        assert_eq!(f.origin_tip(), before, "{refspec}");
    }
    assert!(git(&f.origin, &["for-each-ref", "refs/heads/review-branch"]).is_empty());

    let output = f.publish(&f.task);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output));
    assert_eq!(f.origin_tip(), commit);
    assert_eq!(f.canonical_head(), commit);
}
