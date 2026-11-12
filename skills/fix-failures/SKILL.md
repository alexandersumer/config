---
name: fix-failures
description: Fix real check failures from arguments, pasted errors, local checks, or CI. Use when tests, builds, linters, CI, pipelines, checks, validation, or error output fail and the user wants the root cause fixed with proof.
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Fix a real failing check from `$ARGUMENTS`, pasted output, current-branch CI, or documented local checks. With no arguments, discover the current branch's failing CI/check signal yourself before asking the user for logs.

## Resolve the target

1. A failing command, job, test, assertion, diagnostic, or log named in `$ARGUMENTS` or pasted output is the scope. Fix it first; touch other failures only if they block reproduction or proof.
2. With no supplied failure, use the latest branch-relevant failing CI/check run for the current branch or PR as the source of truth.
3. If CI is unavailable, inaccessible, green, inconclusive, or not relevant to this branch, fall back to documented local checks, package scripts, and build files to find the smallest real failure.
4. If no failing signal is found, stop and report that no actionable failure was found. Do not invent a change.
5. Push/commit only when authorized by the current request or earlier instructions for this task, for example when the user asks to push, publish, commit, or iterate until CI is green. Otherwise finish with local proof and any remote-CI blocker.

## Triage from evidence

Capture the concrete failing signal before editing: CI/check name or URL, command, exit code, test name, assertion, stack trace, diagnostic, log line, timeout signature, or infrastructure error. Prefer current-branch/PR runs over stale runs from other branches.

For multiple failures, fix one evidence-backed root cause at a time, starting with the named target or the first branch-relevant gate that blocks CI. After each fix, reclassify the rest as related, newly introduced, flaky/infrastructure, pre-existing, or unrelated.

Reproduce locally with the narrowest useful command when feasible; if reproduction is CI-only, environment-specific, permission-limited, or too expensive, proceed from CI evidence and say why.

## Fix constraints

Fix the root cause in application code, an incorrect intended-behavior test, fixture, owned config, or owned infrastructure required by this change.

No suppressions, baselines, skipped/deleted/weakened tests, sleeps, broad catches, silent fallbacks, fake success exits, retry wrappers that hide the defect, or code that dodges the checker. Dependency, build config, or test-infra changes are allowed only when the captured evidence proves they are the minimal root-cause fix.

Do not blindly rerun the same command or CI job; each run must follow a fix, narrow the hypothesis, or add a diagnostic. Stop with a concrete blocker instead of looping on inaccessible logs, missing permissions, unavailable CI, or ambiguous product intent.

## Verify

1. Rerun or reuse fresh proof for the targeted failing check, and compare it with the original failing signal. Do not rerun a passing broad proof when no relevant files changed since; run only the narrowest command covering any new edit.
2. Run or reuse the broader relevant suite, build, lint, or CI gate only when the proof policy justifies it; otherwise name the exact broad-proof blocker or narrower proof used.
3. If adding or changing an automated check, prove when feasible that it fails for the original bug, then restore the fix and rerun green; reuse prior fail/pass proof only when it is same-diff and same-scope.
4. When publishing is authorized, publish only a coherent fix. If the requested outcome includes green CI, inspect resulting CI and repeat this loop for new branch-relevant failures until CI is green or a concrete blocker remains. A request to commit or push alone does not add a CI-monitoring task. Claim CI is green only when the refreshed provider data is for the latest branch-relevant source SHA and every visible required branch-relevant check is terminal success; otherwise report the exact pending, red, missing, stale, or inaccessible check state as the blocker.

## Final

- Fixed: `<root cause>`
- Failure source: `<arguments | pasted output | CI job/check | local check>`
- Evidence: `<original failing signal>`
- Changed: `<files>`
- Proof: `<targeted rerun/result or reused proof>`; `<broader CI/local result, reused proof, or blocker>`
- Publish/CI: `<not authorized | pushed commit(s) and CI result | blocker>`
- Remaining: `<none or exact blocker>`
