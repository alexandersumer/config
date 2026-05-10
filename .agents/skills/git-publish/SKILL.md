---
name: git-publish
description: Commit and publish changes
---

Commit current staged, unstaged, and relevant untracked changes, then push to `origin`. Create a PR only when starting from the default branch. If the user did not explicitly request publishing, ask before any git write.

Do not publish with a provider-generated title or branch-name subject.

Workflow:
1. Inspect current branch, remote default branch, status, and the effective publish diff for one subject: staged changes, unstaged changes, and relevant untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs.
2. If no changes, stop.
3. Stage intended changes unless explicitly excluded.
4. If already on a non-default branch: commit and push current branch only. Do not inspect or update PRs unless asked.
5. If on `main`, `master`, or remote default: create a new branch, commit, push, and open a PR with the validated subject as title.

Do not run tests/builds unless explicitly asked.

Subject regex:
`^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`

No branch names, issue-title summaries, issue-key prefixes, chat summaries, provider defaults, or trailing period.

Final: commit hash, subject, branch, push/PR result.
