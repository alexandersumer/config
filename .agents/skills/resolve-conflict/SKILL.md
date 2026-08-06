---
name: resolve-conflict
description: Resolve Git merge conflicts while preserving both branch intents. Use when the repository has unmerged index entries or an in-progress merge with conflicts.
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Resolve conflicts while preserving current branch intent and incorporating incoming changes.

Do not choose one side wholesale just to remove markers.

Read conflict context and enough history from both sides. If incoming removed feature flags, dead code, deprecated APIs, or temporary constructs this branch did not introduce, accept the removal and adapt branch code.

Treat only files reported as unmerged by Git as textual conflicts; do not treat marker text in unrelated fixtures or documentation as a conflict. Resolve those files first. Edit a non-unmerged file only when targeted proof shows that a merge-caused semantic integration failure requires the adjacent change. After editing, search the unmerged files for `<<<<<<<`, `=======`, and `>>>>>>>`; marker search is an additional content check, not proof that Git's index is resolved. Stage each intentional resolution or proven integration fix, then require `git diff --name-only --diff-filter=U` or `git ls-files -u` to be empty. Reuse proof when valid; otherwise run the build or most relevant available check if practical and justified by the proof policy. Do not report resolution while conflict markers, unmerged index entries, or known merge-caused failures remain.

Final:
- Resolved: `<files>`
- Preserved: `<branch intent>`
- Incorporated: `<incoming change>`
- Index: `<no unmerged entries | exact remaining paths>`
- Checks: `<command>` -> `<result>`, `reused — <prior proof and why still valid>`, or `not run — <reason>`
