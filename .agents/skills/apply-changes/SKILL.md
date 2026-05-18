---
name: apply-changes
description: Implement a requested code change without a formal plan. Use when the user asks to edit, tweak, add, remove, rename, or adjust behavior and the work should be completed directly.
register_cmd: true
---

Apply `changes`, else `$ARGUMENTS`, else the conversation request, else the obvious requested change from the current branch/worktree context.

Do not stop at "no changes applied" until you have tried to resolve the target from referenced files, current diff, recent conversation, README/CONTRIBUTING/instructions, nearby tests, callers, and 2-3 sibling patterns. If the target is still unclear, ask one precise question and stop.

Before editing, state the intended behavior in one sentence for yourself, then inspect the real entry point and data/error flow that must change. Make the smallest complete edit that matches existing naming, layering, error handling, tests, and comment density.

A complete edit may include production code, wiring, fixtures, tests, docs, or config when they are required to make the requested behavior reachable and provable. Do not add dependencies, abstractions, broad refactors, suppressions, skipped/weakened tests, fake TODOs, or explanatory comments unless the request requires them.

Prefer fixing the root cause over patching the nearest symptom. If the requested change conflicts with repo conventions, existing behavior, or safety, stop and explain the conflict instead of improvising.

Run the narrowest check that proves the changed behavior. Add or update a targeted check when the repo has an appropriate seam and the change would otherwise be unproven. Run broader checks when the touched surface justifies it. Do not claim fixed, complete, ready, or passing without fresh proof from this turn.

Final:
- Intent: `<requested behavior>`
- Changed: `<files>`
- Proof: `<observable behavior or regression covered>`
- Checks: `<command -> result>` or `not run — <reason>`
- Remaining: `<none or exact blocker>`
