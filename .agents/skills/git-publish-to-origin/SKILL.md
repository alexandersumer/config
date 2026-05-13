---
name: git-publish-to-origin
description: Commit and push current branch
register_cmd: true
---

Commit current staged, unstaged, and relevant untracked changes, then push current branch to `origin`. If the user did not explicitly request publishing, ask before any git write.

Do not create branches, open PRs, run unrequested checks, or use an invalid subject.

Workflow:
1. Inspect branch, status, and the effective publish diff for one subject: staged changes, unstaged changes, and relevant untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs.
2. If no changes, stop.
3. Stage intended changes unless explicitly excluded.
4. Commit with a valid Conventional Commit subject.
5. Push to `origin`, setting upstream if needed.

Do not run tests/builds unless explicitly asked.

Subject regex:
`^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`

No branch names, issue-title summaries, issue-key prefixes, chat summaries, provider defaults, or trailing period.

Final: commit hash, subject, branch, push result.
