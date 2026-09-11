---
name: surgical-edit
description: Implement a requested code change without a formal plan. Use when the user asks to edit, tweak, add, remove, rename, or adjust behavior and the work should be completed directly.
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Apply `changes`, else `$ARGUMENTS`, else the conversation request, else the obvious requested change from the current branch/worktree context.

Resolve the target from the request, referenced files, and current diff. Read callers, tests, or sibling patterns only as needed to settle implementation details. Ask one precise question only if a material ambiguity remains; continue independent work while awaiting the answer.

Trace the smallest complete path from the entry point through the changed behavior to its observable effect and targeted check. For docs, config, mechanical, or test-only edits, identify the artifact, its consumer, and a suitable validation command or direct inspection. Inspect the affected paths and make the smallest complete edit that matches local conventions.

A complete edit may include production code, wiring, fixtures, tests, docs, or config when they are required to make the requested behavior or artifact reachable and provable. Prefer one narrow integrated slice over disconnected preparation work. Use full acceptance/E2E proof only when the request or risk requires it. Add or change tests only when they catch a named realistic regression not already covered by a stronger existing test. Do not add dependencies, abstractions, broad refactors, suppressions, skipped/weakened tests, fake TODOs, or explanatory comments unless the request requires them.

Fix the root cause. A requested behavior change may intentionally replace existing behavior or a local convention. Follow clear user intent; pause only the affected work when a conflict leaves the intended outcome ambiguous or the action requires authorization not already provided.

Validate through the proof policy: reuse proof when valid, otherwise run the narrowest check that proves the changed behavior. Add or update a targeted check when the repo has an appropriate seam and the change would otherwise be unproven. Run broader checks only when justified. Do not claim fixed, complete, ready, or passing without fresh or validly reused proof.

Final:
- Intent: `<requested behavior>`
- Changed: `<files>`
- Proof: `<observable behavior or regression covered>`
- Checks: `<command -> result>`, `reused — <prior proof and why still valid>`, or `not run — <reason>`
- Remaining: `<none or exact blocker>`
