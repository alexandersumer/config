---
name: git-publish
description: Commit current changes and push to origin; create a pull request only from the default branch.
---

Commit current staged, unstaged, and relevant untracked changes, then push to `origin`. Create a pull request only when starting from the default branch.

Do not satisfy this by blindly pushing with a provider-generated title. The known failure mode is damaging review/history hygiene while doing a mechanically successful publish. The branch, commit, and PR title must reflect the actual diff through a valid Conventional Commit subject.

Workflow:
1. Determine current branch, remote default branch, working-tree status, and enough diff context to choose one subject. Do not inspect PRs yet.
2. Do not run tests, builds, linters, type checks, or full verification unless explicitly asked.
3. If there are no staged, unstaged, or relevant untracked changes, stop and say so.
4. Stage intended changes, preserving explicit user exclusions.
5. If current branch is not `main`, not `master`, and not the remote default branch:
   - Do not create, switch, or rename branches.
   - Do not create, inspect, or update PRs unless explicitly asked.
   - Commit with the validated subject as the first line.
   - Push current branch to `origin`, setting upstream if needed.
   - Finish with brief status.
6. If current branch is `main`, `master`, or the remote default branch:
   - Create a new appropriately named branch before committing.
   - Commit with the validated subject as the first line.
   - Push the new branch to `origin`.
   - Create a PR whose title is the validated subject and whose description is concise and diff-grounded.
   - Finish with PR URL and brief status.

Subject rules:
- Match exactly: `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`.
- Format: `<type>[optional scope][!]: <lowercase imperative description>`.
- No trailing period.
- No sentence-case summary, issue title, issue-key prefix, branch name, chat summary, or SCM default.
- If validation fails, fix the subject before any commit, push, or PR creation.
