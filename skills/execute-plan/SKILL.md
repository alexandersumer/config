---
name: execute-plan
description: Implement an existing plan or spec through reachable behavior and verification. Use when the user asks to execute, carry out, or finish a planning artifact.
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Implement `$ARGUMENTS` or the most relevant discovered planning artifact.

Implement every requirement in the requested scope in this turn: the whole artifact unless the user named a phase, chunk, or PR. Do not stop at scaffolding, types, TODOs, docs, unrelated green tests, or a finished first phase. Deliver reachable behavior through real repo entry points.

Before editing:
- read the artifact set end to end
- read applicable repo instructions, touched modules, and relevant tests; consult sibling features when they clarify an implementation choice
- resolve stale paths and equivalent checks against current code when intent is clear; ask only about contradictions or changes that materially affect scope, behavior, or safety
- identify the artifact and briefly note any material adaptation

For multi-step work, track remaining requirements in chat or the available planning tool. Reuse the existing plan rather than duplicating it. Each task should have a checkable outcome.

Build production code, wiring, tests, and required docs/config together. Tests must catch a named realistic regression that is not already covered by a stronger existing test. If no existing check can prove the behavior, add the missing targeted check instead of claiming green. When acceptance needs the running system, prove it with `e2e-automated-tests` or `e2e-live-check` instead of listing it as remaining work.

Change test infrastructure or build config only when needed to implement or verify a planned requirement, using the smallest existing-compatible change. Add production dependencies only when required for planned behavior; broad dependency bumps remain out of scope. Do not add suppressions, baselines, skipped tests, or fake TODO placeholders.

Validate through the proof policy: reuse proof when valid, otherwise run targeted checks before broader checks. Re-read the artifact and account for every in-scope requirement. While one is unimplemented or unverified and nothing outside your control prevents it, keep working. Defer a requirement only when the user descoped it or a concrete blocker prevents it; the plan's length, a failed first attempt, or a finished phase is not a reason. Evidence before claims: no fixed, complete, ready, or passing language without fresh or validly reused proof.

Final under 25 lines:
```text
Artifact: <path or inline>
Implemented:
- <behavior>: <files> — `<command>` -> <result>
Checks: `<command>` -> <result>, `reused — <prior proof and why still valid>`, or `not run — <reason>`
Deferred:
- <item or None>: <user decision or exact blocker>
Remaining: <exact blocker, otherwise none>
```
