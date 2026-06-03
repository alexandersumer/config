---
name: git-publish-to-origin
description: Commit and push current branch
---

Commit current staged, unstaged, and relevant untracked changes, then push the current branch to `origin`.

Invoking this skill is explicit authorization to perform the required git writes for pushing: stage intended changes, create a commit, and push the current branch. Do not pause to ask for publish permission unless the inspected changes are incoherent, risky, or ambiguous.

This skill is push-only: do not create branches, open PRs, inspect PRs, update PRs, run unrequested checks, or use an invalid subject. Stop instead of pushing if the current branch is `main`, `master`, or the resolved remote default branch, unless the user explicitly named that branch as the push target.

## Workflow

1. Inspect repository context before any write:
   - `git rev-parse --show-toplevel`
   - `git branch --show-current`
   - resolved remote default branch from `origin/HEAD`, falling back to `origin/main` or `origin/master` only if needed
   - `git status --short`
   - upstream/ahead state when available: `git rev-parse --abbrev-ref --symbolic-full-name @{u}` and `git rev-list --left-right --count @{u}...HEAD`
   - committed local changes not yet on the upstream or matching `origin/<current-branch>`, when available
   - effective publish diff for one subject:
     - unpushed commit diff when local commits exist
     - staged diff: `git diff --cached`
     - unstaged diff: `git diff`
     - relevant untracked files from `git ls-files --others --exclude-standard`, rendered or summarized as new-file diffs
2. If the current branch is `main`, `master`, or the resolved remote default branch, stop unless the user explicitly named that branch as the push target.
3. If there are no publishable working-tree changes and no unpushed local commits, stop.
4. Confirm the combined effective publish diff forms one coherent push. If unpushed local commits and working-tree changes are unrelated, stop and ask how to split or scope the publish.
5. If working-tree changes exist, stage intended changes unless explicitly excluded, then commit with a valid Conventional Commit subject grounded in those staged changes and compatible with the existing unpushed commits. Do not stage unrelated files.
6. If there are no publishable working-tree changes but the current branch has unpushed local commits, skip committing and push those commits.
7. Push the current branch to `origin`, setting upstream if needed.

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
