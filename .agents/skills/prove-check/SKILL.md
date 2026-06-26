---
name: prove-check
description: Prove a local check catches regressions. Use when validating that a test, script, lint, or gate fails on the bug it claims to catch.
---

## Validation reuse and check scope

Before running a slow, broad, external, stateful, or CI-equivalent command, check whether this conversation or current-SHA CI/artifacts already contain usable proof. Reuse prior passing evidence instead of rerunning only when it is visible, ran after the last relevant edit, covers the same command/scenario and behavior, edge case, or public boundary, and no touched file, config, dependency, fixture, generated output, runtime state, or environment assumption it depends on changed afterward. If uncertain, run the narrowest freshness check that resolves the uncertainty before escalating.

Default to the narrowest honest proof. Run broader suites, full builds, CI reruns, or live/E2E flows only when required by blast radius, merge/release policy, changed shared API/schema/build/test infrastructure/dependencies/auth/security/persistence/concurrency, merge/conflict integration risk, missing targeted seams, or because the broad command is the only proof that covers the behavior.

Final reports must distinguish reused proof, newly run commands, and checks intentionally not run.

Prove `scope`, `$ARGUMENTS`, the relevant recent check, or conversation target catches the regression it claims to catch.

Run or reuse fresh proof for the valid case. If the same check already has visible fail-then-pass proof for the current effective diff and no relevant files changed afterward, reuse that proof instead of reintroducing the regression and report both observed results. Otherwise, introduce one small realistic regression in a fixture, temp copy, or carefully restored edit. Run the same check, or the narrowest command that exercises the same validation path. It must fail for the expected reason. Restore the valid state and run it again.

A clean pass is not proof: like a smoke detector, you only know the check works after it sees the failure it should catch.

If the check cannot fail on a realistic regression, say it is unproven and add or request the smallest check that would prove it.

No suppressions, baselines, skipped checks, dependency bumps, broad config rewrites, or fake exit-code tests.

Final: one short paragraph with the contract, regression, commands/results, changed files, and remaining gap.
