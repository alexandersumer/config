---
name: sync-main
description: Merge latest origin default branch
---

## Validation reuse and check scope

Before running a slow, broad, external, stateful, or CI-equivalent command, check whether this conversation or current-SHA CI/artifacts already contain usable proof. Reuse prior passing evidence instead of rerunning only when it is visible, ran after the last relevant edit, covers the same command/scenario and behavior, edge case, or public boundary, and no touched file, config, dependency, fixture, generated output, runtime state, or environment assumption it depends on changed afterward. If uncertain, run the narrowest freshness check that resolves the uncertainty before escalating.

Default to the narrowest honest proof. Run broader suites, full builds, CI reruns, or live/E2E flows only when required by blast radius, merge/release policy, changed shared API/schema/build/test infrastructure/dependencies/auth/security/persistence/concurrency, merge/conflict integration risk, missing targeted seams, or because the broad command is the only proof that covers the behavior.

Final reports must distinguish reused proof, newly run commands, and checks intentionally not run.

Fetch the latest default branch from `origin` and merge that remote-tracking branch into the current branch. If the user did not explicitly request syncing, ask before any git write.

Stop if the working tree is dirty or the current branch is the default branch. Do not stash, discard, or guess.

Determine the remote default ref from `origin`, not from the local default branch. Keep the resolved value as a full remote-tracking ref such as `origin/main`:
- Prefer `origin/HEAD` after refreshing it if needed.
- Fall back to common remote branches such as `origin/main` or `origin/master` only if `origin/HEAD` cannot be resolved.
- Do not merge local `main`, local `master`, or any other local default branch.

Always fetch before merging. Fetch `origin` so the resolved remote default ref reflects the remote state.

Run `git merge <remote-default-ref>` and inspect status.

If there are conflicts, resolve them inline using the same standard as `resolve-conflict`:
- Preserve current branch intent while incorporating incoming `<remote-default-ref>` changes.
- Do not choose one side wholesale just to remove markers.
- Read conflict context and enough history from both sides to understand the intended merge result.
- Accept incoming removals of feature flags, dead code, deprecated APIs, or temporary constructs this branch did not introduce, then adapt branch code to the removal.
- Search for `<<<<<<<`, `=======`, and `>>>>>>>` after editing.

Reuse fresh prior proof when valid; otherwise run the build or the most relevant available checks if practical and justified by the validation policy. If checks expose merge-caused failures, fix them before completing the merge or report the blocker. Stage resolved files and complete the merge commit only after conflict markers are gone and known merge-caused failures are handled.

Final:
- Synced: `<branch>` with `<remote-default-ref>`
- Merge: `<fast-forward|merge commit|conflicts resolved>`
- Checks: `<command>` -> `<result>`, `reused — <prior proof and why still valid>`, or `not run — <reason>`
