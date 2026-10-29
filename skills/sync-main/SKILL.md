---
name: sync-main
description: Merge latest origin default branch
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Fetch the latest default branch from `origin` and merge that remote-tracking branch into the current branch. If the user did not explicitly request syncing, ask before any git write.

Stop if the working tree is dirty or the current branch is the default branch. Do not stash, discard, or guess.

Determine the remote default ref from `origin`, not from the local default branch. Keep the resolved value as a full remote-tracking ref such as `origin/main`:
- Prefer `origin/HEAD` after refreshing it if needed.
- Fall back to common remote branches such as `origin/main` or `origin/master` only if `origin/HEAD` cannot be resolved.
- Do not merge local `main`, local `master`, or any other local default branch.

Always fetch before merging. Fetch `origin` so the resolved remote default ref reflects the remote state.

Run `git merge --no-commit <remote-default-ref>` and inspect status. A fast-forward may complete immediately because `--no-commit` cannot pause it; otherwise keep `MERGE_HEAD` active until integration checks and fixes are complete.

If there are conflicts, resolve them inline using the same standard as `resolve-conflict`:
- Preserve current branch intent while incorporating incoming `<remote-default-ref>` changes.
- Do not choose one side wholesale just to remove markers.
- Read conflict context and enough history from both sides to understand the intended merge result.
- Accept incoming removals of feature flags, dead code, deprecated APIs, or temporary constructs this branch did not introduce, then adapt branch code to the removal.
- Search the unmerged files for `<<<<<<<`, `=======`, and `>>>>>>>` after editing.
- Stage each intentional resolution and require `git diff --name-only --diff-filter=U` or `git ls-files -u` to be empty.

Reuse proof when valid; otherwise run the build or the most relevant available checks if practical and justified by the proof policy. If checks expose merge-caused failures, fix and stage the intentional integration changes before completing the merge or report the blocker. When `MERGE_HEAD` exists, complete the merge commit only after conflict markers and unmerged index entries are gone and known merge-caused failures are handled.

Final:
- Synced: `<branch>` with `<remote-default-ref>`
- Merge: `<fast-forward|merge commit|conflicts resolved>`
- Checks: `<command>` -> `<result>`, `reused — <prior proof and why still valid>`, or `not run — <reason>`
