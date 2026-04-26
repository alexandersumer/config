---
name: git-publish-to-origin
description: Commit current changes with a conventional commit, then push the current branch to origin
---

Commit the current staged, unstaged, and relevant untracked changes using a conventional commit message, then push the current branch to `origin`.

Inspect the diff and recent commit history to choose the conventional commit type, optional scope, and message. Include staged changes, unstaged tracked changes, and relevant untracked files unless the user explicitly excludes something.

Before committing, choose one canonical Conventional Commit subject.

Subject rules:
- Match exactly: `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`.
- Format: `<type>[optional scope][!]: <lowercase imperative description>`.
- No trailing period.
- Do not use a sentence-case summary, branch name, issue title, issue-key prefix, or SCM/provider default.
- If the subject fails validation, fix it before any commit or push.

Do not create, switch, rename, or publish a new branch. Work only on the current branch. Do not create or update a pull request as part of this prompt.

Before staging or committing anything, determine the current branch and the remote default branch for `origin`. If the current branch is `main`, `master`, or the remote default branch, stop immediately with an error-style message explaining that this prompt refuses to commit or push directly to a default branch. Do not stage files, create a branch, commit, push, or attempt a workaround.

After the branch safety check passes, inspect the working tree, stage the intended changes, commit using the validated subject as the commit message first line, push the current branch to `origin`, and finish with a brief status summary. Do not treat the chat summary as the commit subject.

If there are no changes to commit, stop and say so.
