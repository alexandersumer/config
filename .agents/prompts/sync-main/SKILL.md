---
name: sync-main
description: Fetch the latest default branch and merge it into the current branch
---

Preconditions (stop and report if either fails):
- Working tree has no uncommitted changes. If it does, tell the user to commit or stash first.
- Current branch is not the default branch.

Determine the upstream remote and default branch (prefer the current branch's upstream remote and its HEAD, then common names such as `main`, `master`, or `develop`). Always fetch the resolved default branch first — never skip, even if refs look current.

Run `git merge <remote>/<default-branch>`, then `git status`. Conflicts exist if output contains `CONFLICT`/`Automatic merge failed` or status shows "Unmerged paths" — resolve them. Otherwise done.

On conflicts, resolve so the current branch's intent is preserved while updates from the default branch are incorporated. When the default branch intentionally removed or simplified code (feature flags, dead code, deprecated APIs, temporary constructs) that the current branch did not introduce, accept the removal and adapt the current branch's code to work without it.

After resolving:
- Search every affected file for `<<<<<<<`, `=======`, `>>>>>>>`. Re-resolve any that remain.
- Run the build if a build command is available.
- Stage all resolved files and complete the merge commit.

Acceptance criteria:
- No conflict markers remain.
- `git status` shows a clean merge (committed or fast-forwarded).
- Build passes if a build command exists.
