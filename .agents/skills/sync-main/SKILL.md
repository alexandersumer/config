---
name: sync-main
description: Merge latest origin default branch
---

Fetch the latest default branch from `origin` and merge that remote-tracking branch into the current branch.

Stop if the working tree is dirty or the current branch is the default branch. Do not stash, discard, or guess.

Determine the default branch from `origin`, not from the local default branch:
- Prefer `origin/HEAD` after refreshing it if needed.
- Fall back to common remote branches such as `origin/main` or `origin/master` only if `origin/HEAD` cannot be resolved.
- Do not merge local `main`, local `master`, or any other local default branch.

Always fetch before merging. Fetch `origin` and the resolved default branch so the merge target reflects the remote state.

Run `git merge origin/<default-branch>` and inspect status.

If there are conflicts, resolve them inline using the same standard as `resolve-conflict`:
- Preserve current branch intent while incorporating incoming `origin/<default-branch>` changes.
- Do not choose one side wholesale just to remove markers.
- Read conflict context and enough history from both sides to understand the intended merge result.
- Accept incoming removals of feature flags, dead code, deprecated APIs, or temporary constructs this branch did not introduce, then adapt branch code to the removal.
- Search for `<<<<<<<`, `=======`, and `>>>>>>>` after editing.

Run the build or the most relevant available checks if practical. Stage resolved files and complete the merge commit when conflicts were resolved.

Final:
- Synced: `<branch>` with `origin/<default>`
- Merge: `<fast-forward|merge commit|conflicts resolved>`
- Checks: `<command>` -> `<result>` or `not run — <reason>`
