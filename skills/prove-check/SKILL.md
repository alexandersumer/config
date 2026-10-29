---
name: prove-check
description: Prove a local check catches regressions. Use when validating that a test, script, lint, or gate fails on the bug it claims to catch.
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Prove `scope`, `$ARGUMENTS`, the relevant recent check, or conversation target catches the regression it claims to catch.

Run or reuse fresh proof for the valid case. If the same check already has visible fail-then-pass proof for the current effective diff and no relevant files changed afterward, reuse that proof instead of reintroducing the regression and report both observed results. Otherwise, record the relevant pre-proof status and diff, then introduce one small realistic regression in a fixture, temporary copy, isolated checkout, or carefully restored edit that does not overwrite existing user changes. Run the same check, or the narrowest command that exercises the same validation path. It must fail for the expected reason. Restore the exact pre-proof state, verify the relevant status and diff match, and run the valid case again.

A clean pass is not proof: like a smoke detector, you only know the check works after it sees the failure it should catch.

If the check cannot fail on a realistic regression, say it is unproven and name the smallest check change that would prove it. Implement that change only when the user also asked to fix or harden the check.

No suppressions, baselines, skipped checks, dependency bumps, broad config rewrites, or fake exit-code tests.

Final: one short paragraph with the contract, regression, commands/results, changed files, and remaining gap.
