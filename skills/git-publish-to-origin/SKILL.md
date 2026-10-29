---
name: git-publish-to-origin
description: Commit and push current branch to origin, including main/default when current
---

Commit staged, unstaged, and relevant untracked changes only when needed, then push the current branch to the same branch name on `origin`.

Invoking this skill explicitly authorizes only the git writes required for that publish:

- stage intended working-tree changes
- create one commit when working-tree changes need committing
- push `HEAD` to `origin/<current-branch>` and set upstream when needed

Default-branch publishing is an expected supported path of this skill. This includes `main` and `master`; the branch name alone is not a reason to ask for confirmation.

This skill is push-only: do not create branches, open PRs, inspect PRs, update PRs, run tests or builds, pull, merge, rebase, force-push, switch branches, or change the push target.

Normal path contract: local-first, direct push, push-verified. Use cheap local git state for deterministic push decisions. Do not run remote-default discovery in the normal path. Treat the actual `git push` result as the authoritative network freshness and safety check; if the remote rejects the push, stop and report that exact push blocker instead of doing speculative network discovery.

Do not pause to ask for publish permission unless the inspected changes, repository, remote, or push target are incoherent, risky, or ambiguous.

## Target

Resolve the push target directly as branch `<current-branch>` on `origin`.

Use `git push -u origin HEAD:<current-branch>` for the push. If the remote branch does not exist, that push may create it; this is part of publishing the current branch and is not the same as creating or switching local branches.

Whether the current branch is also the remote default does not change the target or require extra confirmation.

## Workflow

1. Inspect repository context before any write:
   - `git rev-parse --show-toplevel`
   - `git branch --show-current`
   - `git rev-parse --verify HEAD`
   - local push target evidence: `git remote get-url --push --all origin`
   - `git status --short`
   - upstream state when available: `git rev-parse --abbrev-ref --symbolic-full-name @{u}`
   - target state from local refs only:
     - if upstream exists and is `origin/<current-branch>`, use `@{u}` as the effective target
     - else if local ref `refs/remotes/origin/<current-branch>` exists, use `origin/<current-branch>` as the effective target
     - else treat the remote target branch as absent
   - ahead and behind state when the effective target exists: `git rev-list --left-right --count <effective-target>...HEAD`
   - committed local changes not yet on the effective target, when the target exists: `git log --oneline <effective-target>..HEAD`
   - effective publish diff for one subject:
     - unpushed commit diff when local commits exist
     - staged diff: `git diff --cached`
     - unstaged diff: `git diff`
     - relevant untracked files from `git ls-files --others --exclude-standard`, rendered or summarized as new-file diffs
2. Stop before writes if any of these are true:
   - HEAD is detached, `HEAD` has no commit, or the current branch name is empty
   - `origin` is missing
   - the push URL is missing or multiple push URLs make the target ambiguous
   - an existing upstream is not `origin/<current-branch>`
   - local git state says the effective target has commits not in `HEAD`
   - the inspected repository, remote, or diff makes the publish risky or ambiguous for a reason other than the branch name alone
3. If there are no working-tree changes and no commits to publish, stop.
   - Commits to publish exist when `HEAD` has commits not on the effective target.
   - If the remote target branch is absent, a valid `HEAD` counts as publishable because the push publishes the current branch to `origin`.
4. Confirm the combined effective publish diff forms one coherent push. If unpushed commits and working-tree changes are unrelated, stop and ask how to split or scope the publish.
5. If working-tree changes exist, stage only intended changes and commit them with a valid Conventional Commit subject grounded in the staged diff and compatible with any existing unpushed commits. Leave unrelated untracked files unstaged.
6. If there are no working-tree changes but there are unpushed local commits, skip committing and push those commits.
7. Push `HEAD` to branch `<current-branch>` on `origin` with `git push -u origin HEAD:<current-branch>`. If the remote rejects the push, report the exact rejection as the blocker. Do not auto-pull, rebase, force-push, retry with a new strategy, or hide the rejection behind speculative preflight checks.

Do not run tests or builds unless explicitly asked.

## Subject rules

Subject regex:

```text
^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$
```

Rules:

- Use a grounded Conventional Commit subject from the actual diff.
- Do not use branch names, issue-title summaries, issue-key prefixes, chat summaries, provider defaults, or trailing periods.
- Keep the subject human-readable and specific.

## Final response

Report:

- commit hash, or state that no new commit was needed
- subject, or existing unpushed commits when no new commit was needed
- branch
- push result
