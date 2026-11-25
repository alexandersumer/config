//! Direct-to-main publishing for this repository.
//!
//! `publish` is the only supported way to update `origin/main`: one fast-forward
//! push, verified on the remote, then the canonical checkout fast-forwarded to
//! match. The pre-push hook rejects every other push to `origin`, so branches,
//! pull requests, force-pushes, and unsynchronized checkouts cannot happen by
//! omission.
use crate::error::Result;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const REMOTE: &str = "origin";
const REMOTE_REF: &str = "refs/heads/main";
const TRACKING_REF: &str = "refs/remotes/origin/main";
/// Set by `publish` on its own push so the hook can tell it from a bare `git push`.
const PUBLISH_MARKER: &str = "CONFIG_TOOLS_PUBLISH";
/// The push landed, but the canonical checkout could not follow it.
const UNSYNCHRONIZED: u8 = 3;
const IN_PROGRESS: &[(&str, &str)] = &[
    ("MERGE_HEAD", "merge"),
    ("CHERRY_PICK_HEAD", "cherry-pick"),
    ("REVERT_HEAD", "revert"),
    ("rebase-merge", "rebase"),
    ("rebase-apply", "rebase"),
    ("BISECT_LOG", "bisect"),
];

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|err| format!("cannot run git {}: {err}", args.join(" ")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

fn is_ancestor(dir: &Path, ancestor: &str, descendant: &str) -> bool {
    git(dir, &["merge-base", "--is-ancestor", ancestor, descendant]).is_ok()
}

fn short(commit: &str) -> &str {
    commit.get(..12).unwrap_or(commit)
}

fn current_worktree() -> Result<PathBuf> {
    let cwd =
        std::env::current_dir().map_err(|err| format!("cannot read current directory: {err}"))?;
    git(&cwd, &["rev-parse", "--show-toplevel"])
        .map(PathBuf::from)
        .map_err(|_| "publish must run inside a checkout of this repository".to_owned())
}

/// Names the Git operation a checkout is in the middle of, if any.
fn operation_in_progress(dir: &Path) -> Result<Option<&'static str>> {
    for (marker, name) in IN_PROGRESS {
        let path = git(dir, &["rev-parse", "--git-path", marker])?;
        if dir.join(path).exists() {
            return Ok(Some(name));
        }
    }
    Ok(None)
}

fn tracked_changes(dir: &Path) -> Result<bool> {
    git(dir, &["status", "--porcelain", "--untracked-files=no"]).map(|status| !status.is_empty())
}

pub(crate) fn publish_command(args: &[String]) -> Result<ExitCode> {
    if let Some(unknown) = args.first() {
        return Err(format!(
            "unknown option: {unknown}\n\nUsage: config-tools publish\n\nPushes HEAD to {REMOTE}/main as a fast-forward, verifies the remote tip, and fast-forwards the canonical checkout."
        ));
    }
    let worktree = current_worktree()?;
    if let Some(operation) = operation_in_progress(&worktree)? {
        return Err(format!(
            "a {operation} is in progress here; finish or abort it before publishing"
        ));
    }
    if tracked_changes(&worktree)? {
        return Err(
            "this checkout has uncommitted tracked changes; commit or discard them before publishing"
                .into(),
        );
    }

    git(
        &worktree,
        &[
            "fetch",
            "--quiet",
            REMOTE,
            &format!("+{REMOTE_REF}:{TRACKING_REF}"),
        ],
    )
    .map_err(|reason| format!("cannot fetch {REMOTE}/main: {reason}"))?;
    let head = git(&worktree, &["rev-parse", "--verify", "HEAD"])?;
    let mut tip = git(&worktree, &["rev-parse", "--verify", TRACKING_REF])?;

    if is_ancestor(&worktree, &head, &tip) {
        println!(
            "Nothing to publish: {} is already in {REMOTE}/main.",
            short(&head)
        );
    } else if !is_ancestor(&worktree, &tip, &head) {
        return Err(format!(
            "{REMOTE}/main advanced to {} and is not in this checkout. Integrate it here (`git merge {REMOTE}/main`), rerun the affected checks, then publish again.",
            short(&tip)
        ));
    } else {
        let push = Command::new("git")
            .args(["push", REMOTE, &format!("{head}:{REMOTE_REF}")])
            .current_dir(&worktree)
            .env(PUBLISH_MARKER, "1")
            .output()
            .map_err(|err| format!("cannot run git push: {err}"))?;
        if !push.status.success() {
            return Err(format!(
                "{REMOTE} rejected the push. If a concurrent push won, fetch, integrate, revalidate, and publish again.\n{}",
                String::from_utf8_lossy(&push.stderr).trim()
            ));
        }
        let remote = git(&worktree, &["ls-remote", REMOTE, REMOTE_REF])?;
        if remote.split_whitespace().next() != Some(head.as_str()) {
            return Err(format!(
                "{REMOTE}/main does not point at {} after the push; inspect the remote before retrying",
                short(&head)
            ));
        }
        git(&worktree, &["update-ref", TRACKING_REF, &head])?;
        tip = head;
        println!(
            "Published {} to {REMOTE}/main (remote tip verified).",
            short(&tip)
        );
    }

    match synchronize_canonical(&worktree, &tip) {
        Ok(outcome) => {
            println!("{outcome}");
            println!(
                "Verify hosted checks for {} before reporting the work as done.",
                short(&tip)
            );
            Ok(ExitCode::SUCCESS)
        }
        Err(reason) => {
            eprintln!(
                "{REMOTE}/main is at {}, but the canonical checkout was NOT synchronized: {reason}. Leave its local work intact and report this.",
                short(&tip)
            );
            Ok(ExitCode::from(UNSYNCHRONIZED))
        }
    }
}

/// Fast-forwards the primary checkout to `tip`, only when nothing local is at stake.
fn synchronize_canonical(worktree: &Path, tip: &str) -> Result<String> {
    let listing = git(worktree, &["worktree", "list", "--porcelain"])?;
    let canonical = listing
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("worktree "))
        .map(PathBuf::from)
        .ok_or("cannot find the primary checkout")?;
    let label = canonical.display();
    if listing.lines().nth(1) == Some("bare") {
        return Err(format!("{label} is a bare repository"));
    }
    let branch = git(&canonical, &["symbolic-ref", "--quiet", "--short", "HEAD"])
        .map_err(|_| format!("{label} has a detached HEAD"))?;
    if branch != "main" {
        return Err(format!("{label} is on {branch}, not main"));
    }
    if let Some(operation) = operation_in_progress(&canonical)? {
        return Err(format!("{label} has a {operation} in progress"));
    }
    if tracked_changes(&canonical)? {
        return Err(format!("{label} has uncommitted tracked changes"));
    }
    let head = git(&canonical, &["rev-parse", "--verify", "HEAD"])?;
    if head == tip {
        return Ok(format!(
            "Canonical checkout {label} already matches {REMOTE}/main."
        ));
    }
    if !is_ancestor(&canonical, &head, tip) {
        return Err(format!(
            "{label} has local commits that are not in {REMOTE}/main"
        ));
    }
    git(&canonical, &["merge", "--quiet", "--ff-only", tip])
        .map_err(|reason| format!("{label} could not fast-forward: {reason}"))?;
    Ok(format!(
        "Canonical checkout {label} fast-forwarded to {}.",
        short(tip)
    ))
}

/// Git's pre-push hook: `<remote name> <remote url>` as arguments and one
/// `<local ref> <local sha> <remote ref> <remote sha>` line per pushed ref.
pub(crate) fn pre_push_command(args: &[String]) -> Result<()> {
    let [name, url] = args else {
        return Err("usage: config-tools pre-push REMOTE_NAME REMOTE_URL".into());
    };
    let cwd =
        std::env::current_dir().map_err(|err| format!("cannot read current directory: {err}"))?;
    let canonical_url = git(&cwd, &["remote", "get-url", "--push", REMOTE]).ok();
    if name != REMOTE && canonical_url.as_deref() != Some(url.as_str()) {
        return Ok(());
    }
    let mut updates = String::new();
    std::io::stdin()
        .read_to_string(&mut updates)
        .map_err(|err| format!("cannot read pushed refs: {err}"))?;
    let rule = format!(
        "This repository publishes directly to {REMOTE}/main with `cargo run -- publish`; it has no branches or pull requests on {REMOTE}."
    );
    let mut pushing = false;
    for update in updates.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<_> = update.split_whitespace().collect();
        let [_, local, remote_ref, remote] = fields[..] else {
            return Err(format!("unexpected pre-push input: {update}"));
        };
        let absent = |commit: &str| commit.bytes().all(|byte| byte == b'0');
        if remote_ref != REMOTE_REF {
            return Err(format!("refusing to push {remote_ref}. {rule}"));
        }
        if absent(local) {
            return Err(format!("refusing to delete {REMOTE}/main. {rule}"));
        }
        if !absent(remote) && !is_ancestor(&cwd, remote, local) {
            return Err(format!(
                "refusing a push that is not a fast-forward of {REMOTE}/main; never force-push. Fetch, integrate, revalidate, then publish again. {rule}"
            ));
        }
        pushing = true;
    }
    if pushing && std::env::var(PUBLISH_MARKER).as_deref() != Ok("1") {
        return Err(format!(
            "refusing a bare `git push`. Run `cargo run -- publish` instead: it pushes this fast-forward, verifies the remote tip, and fast-forwards the canonical checkout. {rule}"
        ));
    }
    Ok(())
}
