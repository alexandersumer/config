---
name: git-publish-to-origin
description: Commit and push current branch
register_cmd: true
---

Commit current staged, unstaged, and relevant untracked changes, then push the current branch to `origin`.

Invoking this skill is explicit authorization to perform the required git writes for pushing: stage intended changes, create a commit, and push the current branch. Do not pause to ask for publish permission unless the inspected changes are incoherent, risky, or ambiguous.

This skill is push-only: do not create branches, open PRs, inspect PRs, update PRs, run unrequested checks, or use an invalid subject.

## Workflow

1. Inspect repository context before any write:
   - `git rev-parse --show-toplevel`
   - `git branch --show-current`
   - `git status --short`
   - upstream/ahead state when available: `git rev-parse --abbrev-ref --symbolic-full-name @{u}` and `git rev-list --left-right --count @{u}...HEAD`
   - effective publish diff for one subject:
     - staged diff: `git diff --cached`
     - unstaged diff: `git diff`
     - relevant untracked files from `git ls-files --others --exclude-standard`, rendered or summarized as new-file diffs
2. If there are no publishable working-tree changes and no unpushed local commits, stop.
3. If working-tree changes exist, confirm they form one coherent subject. If not, stop and ask how to split or scope the publish.
4. If working-tree changes exist, stage intended changes unless explicitly excluded, then commit with a valid Conventional Commit subject. Do not stage unrelated files.
5. If there are no publishable working-tree changes but the current branch has unpushed local commits, skip committing and push those commits.
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
- commit hash, or state that no new commit was needed
- subject, or existing unpushed commits when no new commit was needed
- branch
- push result
