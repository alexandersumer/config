---
name: resolve-conflict
description: Resolve merge conflicts
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Resolve conflicts while preserving current branch intent and incorporating incoming changes.

Do not choose one side wholesale just to remove markers.

Read conflict context and enough history from both sides. If incoming removed feature flags, dead code, deprecated APIs, or temporary constructs this branch did not introduce, accept the removal and adapt branch code.

Search for `<<<<<<<`, `=======`, `>>>>>>>` after editing. Reuse proof when valid; otherwise run the build or most relevant available check if practical and justified by the proof policy. Do not mark files resolved while conflict markers remain or known merge-caused failures are unfixed.

Final:
- Resolved: `<files>`
- Preserved: `<branch intent>`
- Incorporated: `<incoming change>`
- Checks: `<command>` -> `<result>`, `reused — <prior proof and why still valid>`, or `not run — <reason>`
