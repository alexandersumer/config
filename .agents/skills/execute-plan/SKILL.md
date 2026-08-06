---
name: execute-plan
description: Implement an existing plan or spec through reachable behavior and verification. Use when the user asks to execute, carry out, or finish a planning artifact.
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Implement `artifact`, `spec`, `$ARGUMENTS`, or the most relevant discovered planning artifact.

Do not stop at scaffolding, types, TODOs, docs, or unrelated green tests. Deliver reachable behavior through real repo entry points.

Before editing:
- read the artifact set end to end
- read README/CONTRIBUTING, touched modules, nearest tests, and 2-3 sibling features
- stop instead of improvising if the plan has a blocking contradiction, stale path, impossible check, or unsafe sequence
- emit `Artifact: <path or inline>` and `Canonical patterns: <path> for <aspect>`

Maintain an explicit todo or plan in chat or with the available planning tool. Every task must be an observable behavior or capability with a checkable signal.

Build production code, wiring, tests, and required docs/config together. Tests must catch a named realistic regression that is not already covered by a stronger existing test. If no existing check can prove the behavior, add the missing targeted check instead of claiming green.

Test infrastructure and verification-only build changes are allowed only when the artifact asks for verification infrastructure. Production dependencies or build config may change only when required to make planned behavior reachable, and broad dependency bumps remain out of scope. Do not add suppressions, baselines, skipped tests, or fake TODO placeholders.

Validate through the proof policy: reuse proof when valid, otherwise run targeted checks before broader checks. Re-read the artifact and account for every requirement as implemented or deferred. Evidence before claims: no fixed, complete, ready, or passing language without fresh or validly reused proof.

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
