---
name: sync-main
description: Fetch the latest default branch and merge it into the current branch
---

Fetch the latest default branch and merge it into the current branch.

Do not satisfy this by merging stale refs or hiding local work. The known failure mode is assuming local refs are current, stashing/discarding without permission, or resolving conflicts by undoing default-branch cleanup.

Preconditions. Stop and report if either fails:
- Working tree has no uncommitted changes.
- Current branch is not the default branch.

Determine the upstream remote and default branch, preferring the current branch's upstream remote and its HEAD, then common names such as `main`, `master`, or `develop`. Always fetch the resolved default branch first.

Run `git merge <remote>/<default-branch>`, then `git status`.

If conflicts occur:
- Resolve them so current branch intent remains intact and default-branch updates are incorporated.
- If default branch intentionally removed feature flags, dead code, deprecated APIs, or temporary constructs that this branch did not introduce, accept the removal and adapt branch code.
- Search for `<<<<<<<`, `=======`, `>>>>>>>` after editing.
- Run the build if available.
- Stage resolved files and complete the merge commit.

Done means:
- no conflict markers remain
- `git status` shows a clean fast-forward or committed merge
- build passes if a build command exists

Final response:
- Synced: `<current branch>` with `<remote>/<default-branch>`
- Merge: `<fast-forward|merge commit|conflicts resolved>`
- Checks: `<command>` -> `<result>`
