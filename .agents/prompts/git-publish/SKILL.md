---
name: git-publish
description: Commit current changes and push to origin; create a pull request only from the default branch.
---

Commit the current staged, unstaged, and relevant untracked changes, then push to `origin`. Create a pull request only when starting from the default branch.

Workflow:
1. Determine the current branch, remote default branch, working-tree status, and enough diff context to choose one Conventional Commit subject. Do not inspect pull requests yet.
2. If there are no staged, unstaged, or relevant untracked changes to commit, stop and say so.
3. Stage the intended changes, including staged changes, unstaged tracked changes, and relevant untracked files unless the user explicitly excludes something.
4. If the current branch is not `main`, not `master`, and not the remote default branch:
   - Do not create, switch, or rename branches.
   - Do not create, inspect, or update pull requests unless the user explicitly asks.
   - Commit using the validated subject as the commit message first line.
   - Push the current branch to `origin`. If needed, set upstream tracking for the current branch on `origin`.
   - Finish with a brief status summary.
5. If the current branch is `main`, `master`, or the remote default branch:
   - Create a new appropriately named branch before committing.
   - Choose a branch-level Conventional Commit subject for the pull request title.
   - Commit using the validated commit subject as the commit message first line.
   - Push the new branch to `origin`.
   - Create a pull request with the validated title and a concise description.
   - Finish with the pull request URL and a brief status summary.

Subject rules:
- Match exactly: `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`.
- Format: `<type>[optional scope][!]: <lowercase imperative description>`.
- No trailing period.
- Do not use a sentence-case summary, branch name, issue title, issue-key prefix, chat summary, or SCM/provider default.
- If the subject fails validation, fix it before any commit, push, or pull request creation.
