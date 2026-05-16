---
name: fix-failures
description: Fix real check failures from arguments, pasted errors, local checks, or CI. Use when tests, builds, linters, CI, pipelines, checks, validation, or error output fail and the user wants the root cause fixed with proof.
register_cmd: true
---

Fix the failure from `error_output`, `$ARGUMENTS`, CI, or the repo's checks.

Priority:
1. If `error_output` or `$ARGUMENTS` names a failure, fix that exact failure first. Do not wander to unrelated CI/local failures unless they block reproducing or proving the requested fix.
2. If no failure is provided, inspect the current branch's CI/check status using the available repository/SCM tools. Treat failing CI as the source of truth for what to fix.
3. If CI is unavailable, inconclusive, or has no relevant failure, use the repo's documented local checks to discover the failure.

Work from evidence, not guesses. Capture the failing job, command, test, assertion, stack trace, log line, or diagnostic that explains the defect. Reproduce locally with the smallest useful command when feasible; if local reproduction is impossible, use the CI evidence and say why.

Do not keep rerunning the same command. Each run must change code, narrow the hypothesis, or exercise a new diagnostic. Do not claim fixed, clean, ready, or passing without fresh proof from this turn.

Fix the root cause in application code, tests that assert intended behavior incorrectly, or configuration/infrastructure owned by this change. Avoid patching symptoms.

Do not use suppressions, baselines, skipped/deleted/weakened tests, sleeps, broad catches, silent fallbacks, or wrapper code that dodges the checker. Dependency, build config, or test-infra changes are acceptable only when the failure evidence proves they are the root cause and the change is minimal.

Verification:
1. Rerun the targeted failing check, test, job-equivalent command, or CI job when available.
2. Prove the specific failure is gone by comparing the original failing signal with the new passing signal.
3. Run the broader relevant suite or confirm CI is green after targeted proof passes. If broad proof or CI cannot be run, name the exact blocker.
4. If a new automated check is added or changed, prove when feasible that it would fail for the original bug, then restore and rerun green.

Final:
- Fixed: `<root cause>`
- Failure source: `<arguments | pasted output | CI job/check | local check>`
- Evidence: `<original failing signal>`
- Changed: `<files>`
- Proof: `<targeted rerun/result>`; `<broader CI/local result or blocker>`
- Remaining: `<none or exact blocker>`
