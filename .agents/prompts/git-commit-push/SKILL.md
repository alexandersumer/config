---
name: git-commit-push
description: Commit current changes with a conventional commit, push the branch, and create or update the pull request. Use for PR-oriented commit/push workflows.
---

Commit the current staged, unstaged, and relevant untracked changes, push the working branch, and create or update its pull request.

Inspect the diff, branch state, remotes, recent commits, and existing pull requests to choose the branch, remote, target branch, PR title, and PR description. Use available SCM/code-review tools when they are safer than raw git/provider CLI commands.

Workflow:
1. Inspect the repository state: current branch, remotes, default branch, staged changes, unstaged changes, untracked files, and whether a pull request already exists for the branch.
2. If there are no staged, unstaged, or relevant untracked changes to commit, stop and say so.
3. If the current branch is `main`, `master`, or the remote default branch, create a new appropriately named branch before committing. Do not commit directly to the default branch.
4. Include staged changes, unstaged tracked changes, and relevant untracked files unless the user explicitly excludes something.
5. Commit with a conventional commit message (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`, etc.) that matches the actual diff. Keep the subject concise and imperative.
6. Push the current branch. If a pull request already exists for the branch, reuse it; otherwise create one.
7. Finish with the PR URL and a brief summary of what was committed, pushed, and created or reused.
