---
name: git-publish-to-origin
description: Commit and push current branch to origin, including main/default when current
---

Commit staged, unstaged, and relevant untracked changes only when needed, then push the current branch to the same branch name on `origin`.

Invoking this skill authorizes only these git writes: stage intended changes, create one commit when needed, and push `HEAD` to `origin/<current-branch>` with upstream set when needed.

Publishing `main`, `master`, or the remote default is an expected path; the branch name alone is never a reason to ask for confirmation.

Apart from the CI and review-comment follow-through in step 7, this skill is push-only: do not create branches, open, inspect, or update PRs, run tests or builds, pull, merge, rebase, force-push, switch branches, or change the push target.

Decide from local git state only; do not run remote-default discovery. The `git push` result is the authoritative freshness and safety check: if the remote rejects it, stop and report that rejection rather than doing speculative network discovery.

Ask for permission only when the changes, repository, remote, or push target are incoherent, risky, or ambiguous.

## Target

Push with `git push -u origin HEAD:<current-branch>`. Creating the remote branch this way is part of publishing, not creating or switching local branches. Whether the branch is also the remote default does not change the target.

## Workflow

1. Before any write, inspect the current branch, `HEAD`, status, `git remote get-url --push --all origin`, and upstream (`@{u}`). Pick the effective target from local refs only: `@{u}` if it is `origin/<current-branch>`, else local `refs/remotes/origin/<current-branch>` if it exists, else treat the remote branch as absent. When a target exists, get ahead/behind counts and unpushed commits against it. The publish diff is unpushed commits plus staged, unstaged, and relevant untracked changes.
2. Stop before writes if:
   - HEAD is detached or has no commit, or the branch name is empty
   - `origin` is missing, or its push URL is missing or multiple
   - an existing upstream is not `origin/<current-branch>`
   - local refs show the effective target has commits not in `HEAD`
   - the repository, remote, or diff makes the publish risky or ambiguous for a reason other than the branch name
3. If there are no working-tree changes and no commits to publish, stop. When the remote branch is absent, a valid `HEAD` counts as publishable.
4. If unpushed commits and working-tree changes are unrelated, stop and ask how to split or scope the publish.
5. If working-tree changes exist, stage only intended changes, leaving unrelated untracked files out, and commit with a subject grounded in the staged diff and compatible with existing unpushed commits. Otherwise push the existing unpushed commits.
6. Push with `git push -u origin HEAD:<current-branch>`. On rejection, report the exact rejection. Do not auto-pull, rebase, force-push, or retry with another strategy.
7. Follow through on the push; do not rely on the host to watch it. If the pushed branch has an open PR, run `clear-merge-blockers` on it, which fixes red CI and addresses its unresolved review comments in one loop. Otherwise, if the push triggers CI, watch the pushed SHA until its checks finish; on failure, fix the root cause with `fix-failures`, commit and push the same way, and repeat until green or an exact blocker remains.

Do not run tests or builds before pushing unless explicitly asked.

## Subject rules

```text
^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$
```

Ground the Conventional Commit subject in the actual diff. Never use branch names, issue-title summaries, issue-key prefixes, chat summaries, provider defaults, or trailing periods.

## Final response

Report:

- commit hash, or state that no new commit was needed
- subject, or existing unpushed commits when no new commit was needed
- branch
- push result
- CI result: green, no CI triggered, or the exact remaining blocker
- Comments: addressed, none actionable, no open PR, or the exact remaining blocker
