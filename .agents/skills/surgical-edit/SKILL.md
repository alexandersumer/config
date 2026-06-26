---
name: surgical-edit
description: Implement a requested code change without a formal plan. Use when the user asks to edit, tweak, add, remove, rename, or adjust behavior and the work should be completed directly.
---

## Validation reuse and check scope

Before running a slow, broad, external, stateful, or CI-equivalent command, check whether this conversation or current-SHA CI/artifacts already contain usable proof. Reuse prior passing evidence instead of rerunning only when it is visible, ran after the last relevant edit, covers the same command/scenario and behavior, edge case, or public boundary, and no touched file, config, dependency, fixture, generated output, runtime state, or environment assumption it depends on changed afterward. If uncertain, run the narrowest freshness check that resolves the uncertainty before escalating.

Default to the narrowest honest proof. Run broader suites, full builds, CI reruns, or live/E2E flows only when required by blast radius, merge/release policy, changed shared API/schema/build/test infrastructure/dependencies/auth/security/persistence/concurrency, merge/conflict integration risk, missing targeted seams, or because the broad command is the only proof that covers the behavior.

Final reports must distinguish reused proof, newly run commands, and checks intentionally not run.

Apply `changes`, else `$ARGUMENTS`, else the conversation request, else the obvious requested change from the current branch/worktree context.

Do not stop at "no changes applied" until you have tried to resolve the target from referenced files, current diff, recent conversation, README/CONTRIBUTING/instructions, nearby tests, callers, and 2-3 sibling patterns. If the target is still unclear, ask one precise question and stop.

Before editing, state the intended steel thread in one sentence for yourself. For behavior changes, a steel thread is the smallest real path that makes the requested outcome reachable and checkable: entry point → core behavior/state/error path → caller or user-visible effect → targeted proof. For mechanical, docs, config, or test-only requests, use the smallest checkable artifact contract instead: requested artifact change → consumer/location that uses it → validation command or reviewable proof. Then inspect only the real entry point, data/error flow, caller, artifact consumer, and validation seam that must change. Make the smallest complete edit that matches existing naming, layering, error handling, tests, and comment density.

A complete edit may include production code, wiring, fixtures, tests, docs, or config when they are required to make the requested behavior or artifact reachable and provable. Prefer one narrow integrated slice over disconnected preparation work. A steel thread is an implementation slice, not automatically a full real E2E; use full acceptance/E2E proof only when the request or risk requires it. Do not add dependencies, abstractions, broad refactors, suppressions, skipped/weakened tests, fake TODOs, or explanatory comments unless the request requires them.

Prefer fixing the root cause over patching the nearest symptom. If the requested change conflicts with repo conventions, existing behavior, or safety, stop and explain the conflict instead of improvising.

Validate through the reuse/scope policy: reuse fresh prior proof when valid, otherwise run the narrowest check that proves the changed behavior. Add or update a targeted check when the repo has an appropriate seam and the change would otherwise be unproven. Run broader checks only when justified. Do not claim fixed, complete, ready, or passing without fresh proof from this conversation or current-SHA artifacts.

Final:
- Intent: `<requested behavior>`
- Changed: `<files>`
- Proof: `<observable behavior or regression covered>`
- Checks: `<command -> result>`, `reused — <prior proof and why still valid>`, or `not run — <reason>`
- Remaining: `<none or exact blocker>`
