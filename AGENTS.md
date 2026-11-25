# Working on this repository

- `origin/main` is canonical. This repository has no branches or pull requests on `origin`: validated commits go straight to `origin/main`.
- Fetch `origin/main` and create one detached worktree per task from it, and reuse it for follow-ups. Keep the canonical checkout (`~/src/config`) and other sessions' worktrees free of task edits.
- Stage only the files for the task. Every commit, including a merge, runs the full check in its Git hook; never bypass it.
- Publish with `cargo run -- publish` from the task worktree. It pushes HEAD to `origin/main` only as a fast-forward, verifies the remote tip, and fast-forwards the canonical checkout. The pre-push hook rejects a bare `git push`, any other branch, a deletion, and a force-push.
- If publish reports that `origin/main` advanced, run `git merge origin/main` in the task worktree, resolve overlaps while preserving both changes, rerun affected checks, and publish again. If an overlap needs a decision, stop and report the conflicting changes.
- If publish exits 3, the push landed but the canonical checkout has local work or is off `main`. Leave that work intact and report it.
- After publishing, verify GitHub checks for the published commit: `gh run list --commit <sha>`, then `gh run watch <run-id> --exit-status`. Fix task-related failures through the same workflow, then remove the task worktree.
