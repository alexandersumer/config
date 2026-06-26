---
name: execute-plan
description: Implement a plan
---

## Validation reuse and check scope

Before running a slow, broad, external, stateful, or CI-equivalent command, check whether this conversation or current-SHA CI/artifacts already contain usable proof. Reuse prior passing evidence instead of rerunning only when it is visible, ran after the last relevant edit, covers the same command/scenario and behavior, edge case, or public boundary, and no touched file, config, dependency, fixture, generated output, runtime state, or environment assumption it depends on changed afterward. If uncertain, run the narrowest freshness check that resolves the uncertainty before escalating.

Default to the narrowest honest proof. Run broader suites, full builds, CI reruns, or live/E2E flows only when required by blast radius, merge/release policy, changed shared API/schema/build/test infrastructure/dependencies/auth/security/persistence/concurrency, merge/conflict integration risk, missing targeted seams, or because the broad command is the only proof that covers the behavior.

Final reports must distinguish reused proof, newly run commands, and checks intentionally not run.

Implement `artifact`, `spec`, `$ARGUMENTS`, or the most relevant discovered planning artifact.

Do not stop at scaffolding, types, TODOs, docs, or unrelated green tests. Deliver reachable behavior through real repo entry points.

Before editing:
- read the artifact set end to end
- read README/CONTRIBUTING, touched modules, nearest tests, and 2-3 sibling features
- stop instead of improvising if the plan has a blocking contradiction, stale path, impossible check, or unsafe sequence
- emit `Artifact: <path or inline>` and `Canonical patterns: <path> for <aspect>`

Maintain an explicit todo or plan in chat or with the available planning tool. Every task must be an observable behavior or capability with a checkable signal.

Build production code, wiring, tests, and required docs/config together. Tests must catch a named realistic regression. If no existing check can prove the behavior, add the missing targeted check instead of claiming green.

Test infra/build config changes are allowed only when the artifact asks for verification infrastructure. Otherwise no suppressions, baselines, dependency bumps, build config edits, skipped tests, or fake TODO placeholders.

Validate through the reuse/scope policy: reuse fresh prior proof when valid, otherwise run targeted checks before broader checks. Re-read the artifact and account for every requirement as implemented or deferred. Evidence before claims: no fixed, complete, ready, or passing language without fresh proof from this conversation or current-SHA artifacts.

Final under 25 lines:
```text
Artifact: <path or inline>
Implemented:
- <behavior>: <files> — `<command>` -> <result>
Checks: `<command>` -> <result>, `reused — <prior proof and why still valid>`, or `not run — <reason>`
Deferred:
- <item or None>: <reason>
Next: review-solo or review-deep as appropriate, then git-publish.
```
