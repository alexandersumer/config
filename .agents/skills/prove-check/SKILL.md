---
name: prove-check
description: Prove a local check catches regressions. Use when validating that a test, script, lint, or gate fails on the bug it claims to catch.
register_cmd: true
---

Prove `scope`, `$ARGUMENTS`, the relevant recent check, or conversation target catches the regression it claims to catch.

Run the valid case. Introduce one small realistic regression in a fixture, temp copy, or carefully restored edit. Run the same check, or the narrowest command that exercises the same validation path. It must fail for the expected reason. Restore the valid state and run it again.

A clean pass is not proof: like a smoke detector, you only know the check works after it sees the failure it should catch.

If the check cannot fail on a realistic regression, say it is unproven and add or request the smallest check that would prove it.

No suppressions, baselines, skipped checks, dependency bumps, broad config rewrites, or fake exit-code tests.

Final: one short paragraph with the contract, regression, commands/results, changed files, and remaining gap.
