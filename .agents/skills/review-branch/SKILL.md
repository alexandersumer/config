---
name: review-branch
description: Review branch defects
---

Review `git diff base...HEAD`, narrowed by `focus_area` or `$ARGUMENTS` if provided.

Look past the patch shape: trace changed invariants, edge inputs, failure paths, concurrency/idempotency, auth/data boundaries, persistence/API contracts, rollout paths, and whether new seams violate existing architecture.

Report only material, actionable issues: correctness, security, broken invariants, serious architecture leaks, missing tests for likely regressions, or missing rollout safety for risky runtime behavior. Prefer one high-confidence severe finding over many minor notes.

Do not report style, formatting, import order, naming taste, speculative risks, or mitigated issues.

Output one finding per line:
`<file>:<line> <problem> -> <fix>`

If none: `no issues found`
