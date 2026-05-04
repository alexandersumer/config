---
name: git-publish-to-origin
description: Commit current changes and push to origin.
---

Commit the current staged, unstaged, and relevant untracked changes, then push the current branch to `origin`.

Do not satisfy this by creating branches, opening PRs, running surprise verification, or using a sloppy commit message. The known failure mode is letting the SCM/provider default or branch name become history. The commit subject must be a valid Conventional Commit before any write action.

Workflow:
1. Determine current branch, working-tree status, and enough diff context to choose one subject.
2. Do not run tests, builds, linters, type checks, or full verification unless the user explicitly asks.
3. If there are no staged, unstaged, or relevant untracked changes, stop and say so.
4. Stage intended changes, preserving explicit user exclusions.
5. Commit using the validated subject as the first line.
6. Push the current branch to `origin`, setting upstream if needed.
7. Finish with a brief status summary.

Do not create, switch, or rename branches. Do not create, inspect, or update pull requests.

Subject rules:
- Match exactly: `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`.
- Format: `<type>[optional scope][!]: <lowercase imperative description>`.
- No trailing period.
- No sentence-case summary, issue title, issue-key prefix, branch name, chat summary, or SCM default.
- If validation fails, fix the subject before committing.
