---
name: fix-failures
description: Run build/test, fix failures without workarounds or suppressing errors
argument-hint: "[optional: error output or stack trace]"
inputs:
  - name: error_output
    label: Error output or stack trace
    description: Paste error output or stack trace to fix. Leave empty to run the build and test suite to discover failures.
    type: string
    required: false
---

Read the README to learn how to run checks locally.

Use the error output below if provided; otherwise, run the full local check suite to discover failures.

$ARGUMENTS

For each failure:
1. Reproduce locally so you can see it (mandatory for integration tests).
2. Diagnose the root cause in the application code under test.
3. Fix the application code the checker is pointing at. Examples of correct fixes: remove a redundant `suspend` modifier when a linter flags it; rewrite an unsafe pattern when a static analyzer flags it; correct the production logic when a test asserts the right behavior.

Allowed changes: application source code, test code that asserts correct behavior.
Disallowed changes: `@Suppress`, `@SuppressWarnings`, `noinspection`, lint/detekt/checker baselines, annotation-based opt-outs, wrapping code to dodge a checker, dependency version bumps, build config changes, test infrastructure changes.

After each fix, re-run the full local check suite. Iterate until green.

Acceptance criteria:
- Local check suite passes end-to-end.
- No file in the disallowed list above was modified.
- If failures remain after 3 fix attempts, stop and report each remaining failure with its diagnosis.
