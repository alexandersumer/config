---
name: resolve-conflict
description: Resolve merge conflicts
---

## Validation reuse and check scope

Before running a slow, broad, external, stateful, or CI-equivalent command, check whether this conversation or current-SHA CI/artifacts already contain usable proof. Reuse prior passing evidence instead of rerunning only when it is visible, ran after the last relevant edit, covers the same command/scenario and behavior, edge case, or public boundary, and no touched file, config, dependency, fixture, generated output, runtime state, or environment assumption it depends on changed afterward. If uncertain, run the narrowest freshness check that resolves the uncertainty before escalating.

Default to the narrowest honest proof. Run broader suites, full builds, CI reruns, or live/E2E flows only when required by blast radius, merge/release policy, changed shared API/schema/build/test infrastructure/dependencies/auth/security/persistence/concurrency, merge/conflict integration risk, missing targeted seams, or because the broad command is the only proof that covers the behavior.

Final reports must distinguish reused proof, newly run commands, and checks intentionally not run.

Resolve conflicts while preserving current branch intent and incorporating incoming changes.

Do not choose one side wholesale just to remove markers.

Read conflict context and enough history from both sides. If incoming removed feature flags, dead code, deprecated APIs, or temporary constructs this branch did not introduce, accept the removal and adapt branch code.

Search for `<<<<<<<`, `=======`, `>>>>>>>` after editing. Reuse fresh prior proof when valid; otherwise run the build or most relevant available check if practical and justified by the validation policy. Do not mark files resolved while conflict markers remain or known merge-caused failures are unfixed.

Final:
- Resolved: `<files>`
- Preserved: `<branch intent>`
- Incorporated: `<incoming change>`
- Checks: `<command>` -> `<result>`, `reused — <prior proof and why still valid>`, or `not run — <reason>`
