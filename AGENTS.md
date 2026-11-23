# Working on this repository

- Treat `origin/main` as canonical. Publish completed, validated changes directly to `origin/main`. Do not open pull requests.
- Fetch `origin/main` before starting. Create a dedicated worktree from that ref for each task and reuse it for follow-ups. Keep the canonical checkout (`~/src/config`) and other sessions' worktrees free of task edits.
- Stage only the files for the task. Before publishing, fetch again and check that your commit includes the latest `origin/main`. If origin advanced, integrate its changes in your task worktree, resolve overlaps while preserving both changes, and rerun affected checks.
- Use a normal fast-forward push (for a detached worktree, `git push origin HEAD:refs/heads/main`). Never force-push. If a concurrent push wins, fetch, integrate, revalidate, and retry. If an overlap needs a decision, stop and report the conflicting changes.
- After pushing, check the remote tip and confirm it contains your commit. Fast-forward the canonical checkout only if it is still clean, on `main`, and has no local commits or in-progress operation. Use `git merge --ff-only origin/main` after fetching. Leave concurrent local work intact and report any divergence that prevents synchronization.
- Verify GitHub checks for the published commit when CI runs. Fix task-related failures through the same direct-to-origin workflow.
