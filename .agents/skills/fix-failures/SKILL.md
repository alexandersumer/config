---
name: fix-failures
description: Fix real check failures from arguments, pasted errors, local checks, or CI. Use when tests, builds, linters, CI, pipelines, checks, validation, or error output fail and the user wants the root cause fixed with proof.
---

Fix a real failing check from `error_output`, `$ARGUMENTS`, current-branch CI, or documented local checks. With no arguments, discover the current branch's failing CI/check signal yourself before asking the user for logs.

## Resolve the target

1. If `error_output` or `$ARGUMENTS` names a failing command, job, test, assertion, diagnostic, or pasted log, that is the scope. Fix it first; do not chase unrelated failures unless they block reproduction or proof.
2. With no supplied failure, inspect repository and SCM context: current branch, upstream or PR, working tree diff, recent commits, and branch/PR check status. Use the latest branch-relevant failing CI/check run as the source of truth.
3. If CI is unavailable, inaccessible, green, inconclusive, or not relevant to this branch, fall back to documented local checks, package scripts, build files, and nearby test commands to find the smallest real failure.
4. If no failing signal is found after CI and local discovery, stop and report that no actionable failure was found. Do not invent a change.
5. Push/commit only when this invocation explicitly authorizes it, for example when the user asks to push, publish, commit, or iterate until CI is green. Otherwise finish with local proof and any remote-CI blocker.

## Triage from evidence

Capture the concrete failing signal before editing: CI/check name or URL, command, exit code, test name, assertion, stack trace, compiler/linter diagnostic, log line, timeout signature, or infrastructure error. Prefer current-branch/PR runs over stale runs from other branches.

For multiple failures, group by likely root cause and fix one evidence-backed cause at a time. Start with the named target or the first branch-relevant gate that blocks CI. After each fix, re-check whether remaining failures are related, newly introduced, flaky/infrastructure noise, pre-existing, or unrelated. Expand scope only when needed to prove the requested fix.

Reproduce locally with the narrowest useful command when feasible. If reproduction is CI-only, environment-specific, permission-limited, or too expensive, proceed from CI evidence and say why local reproduction was skipped.

## Fix constraints

Inspect the real seam before editing: failing command or entry point, failing code/config path, relevant state/model, representative caller or consumer, and adjacent tests or sibling patterns.

Fix the root cause in application code, an incorrect intended-behavior test, fixture, owned config, or owned infrastructure required by this change. Do not patch symptoms.

No suppressions, baselines, skipped/deleted/weakened tests, sleeps, broad catches, silent fallbacks, fake success exits, retry wrappers that hide the defect, or code that dodges the checker. Dependency, build config, or test-infra changes are allowed only when the captured evidence proves they are the minimal root-cause fix.

Do not blindly rerun the same command or CI job. Each run must follow a fix, narrow the hypothesis, or exercise a new diagnostic. Stop with a concrete blocker instead of looping on inaccessible logs, missing permissions, unavailable CI, or ambiguous product intent.

## Verify

1. Rerun the targeted failing check, test, job-equivalent command, or CI job when available.
2. Compare the original failing signal with the new passing signal or changed diagnostic.
3. Run the broader relevant suite, build, lint, or CI gate when touched surface justifies it; otherwise name the exact broad-proof blocker.
4. If adding or changing an automated check, prove when feasible that it fails for the original bug, then restore the fix and rerun green.
5. When publish is authorized by this invocation, publish only a coherent fix, inspect resulting CI, and repeat this evidence-backed loop for new branch-relevant failures until CI is green or a concrete blocker remains.

## Final

- Fixed: `<root cause>`
- Failure source: `<arguments | pasted output | CI job/check | local check>`
- Evidence: `<original failing signal>`
- Changed: `<files>`
- Proof: `<targeted rerun/result>`; `<broader CI/local result or blocker>`
- Publish/CI: `<not authorized | pushed commit(s) and CI result | blocker>`
- Remaining: `<none or exact blocker>`
