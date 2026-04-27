---
name: git-publish-to-origin
description: Commit current changes and push to origin.
---

Commit the current staged, unstaged, and relevant untracked changes, then push to `origin`.

Workflow:
1. Determine the current branch, working-tree status, and enough diff context to choose one Conventional Commit subject.
2. If there are no staged, unstaged, or relevant untracked changes to commit, stop and say so.
3. Stage the intended changes, including staged changes, unstaged tracked changes, and relevant untracked files unless the user explicitly excludes something.
4. Commit using the validated subject as the commit message first line.
5. Push the current branch to `origin`. If needed, set upstream tracking for the current branch on `origin`.
6. Finish with a brief status summary.

Do not create, switch, or rename branches. Do not create, inspect, or update pull requests.

Subject rules:
- Match exactly: `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`.
- Format: `<type>[optional scope][!]: <lowercase imperative description>`.
- No trailing period.
- Do not use a sentence-case summary, branch name, issue title, issue-key prefix, chat summary, or SCM/provider default.
- If the subject fails validation, fix it before any commit or push.
