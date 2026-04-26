---
name: git-publish
description: Commit current changes with a conventional commit, push the branch, and create or update the pull request.
---

Commit the current staged, unstaged, and relevant untracked changes, push the working branch, and create or update its pull request.

Inspect the diff, branch state, remotes, recent commits, and existing pull requests to choose the branch, remote, target branch, pull request title, and pull request description. Use available SCM/code-review tools when they are safer than raw git/provider CLI commands.

Before committing or creating/updating a pull request, choose one canonical Conventional Commit subject. The commit subject and pull request title must be exactly identical.

Subject rules:
- Match exactly: `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`.
- Format: `<type>[optional scope][!]: <lowercase imperative description>`.
- No trailing period.
- Do not use a sentence-case summary, branch name, issue title, issue-key prefix, or SCM/provider default.
- If the subject or pull request title fails validation, fix it before any commit, push, pull request create, or pull request update.

Workflow:
1. Inspect the repository state: current branch, remotes, default branch, staged changes, unstaged changes, untracked files, and whether a pull request already exists for the branch.
2. If there are no staged, unstaged, or relevant untracked changes to commit, stop and say so.
3. If the current branch is `main`, `master`, or the remote default branch, create a new appropriately named branch before committing. Do not commit directly to the default branch.
4. Include staged changes, unstaged tracked changes, and relevant untracked files unless the user explicitly excludes something.
5. Commit using the validated subject as the commit message first line. Add a body only when useful.
6. Push the branch. Reuse an existing pull request or create one. Set the pull request title to the exact validated subject.
7. Finish with the pull request URL and a brief status summary. Do not treat the chat summary as the commit subject or pull request title.
