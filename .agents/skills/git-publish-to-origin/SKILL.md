---
name: git-publish-to-origin
description: Commit and push current branch
register_cmd: true
---

Commit current staged, unstaged, and relevant untracked changes, then push the current branch to `origin`. If the user did not explicitly request publishing, ask before any git write.

This skill is push-only: do not create branches, open PRs, inspect PRs, update PRs, run unrequested checks, or use an invalid subject.

## Workflow

1. Inspect repository context before any write:
   - `git rev-parse --show-toplevel`
   - `git branch --show-current`
   - `git status --short`
   - effective publish diff for one subject:
     - staged diff: `git diff --cached`
     - unstaged diff: `git diff`
     - relevant untracked files from `git ls-files --others --exclude-standard`, rendered or summarized as new-file diffs
2. If there are no publishable changes, stop.
3. Confirm the changes form one coherent subject. If not, stop and ask how to split or scope the publish.
4. Stage intended changes unless explicitly excluded. Do not stage unrelated files.
5. Commit with a valid Conventional Commit subject.
6. Push the current branch to `origin`, setting upstream if needed.

Do not run tests/builds unless explicitly asked.

## Subject rules

Subject regex:

```text
^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$
```

Rules:
- Use a grounded Conventional Commit subject from the actual diff.
- No branch names, issue-title summaries, issue-key prefixes, chat summaries, provider defaults, or trailing period.
- Keep the subject human-readable and specific.

## Final response

Report:
- commit hash
- subject
- branch
- push result
