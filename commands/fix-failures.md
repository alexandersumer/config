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

First, find and read the README to learn how to run checks locally — do not skip this.

Use the error output below if provided; otherwise, run the full local check suite to identify failures. Before fixing, reproduce the failure locally to confirm you can see it — this is especially important for integration tests. Diagnose the root cause. Fix it with a clean, architectural solution — fix the actual code that the checker is complaining about. Test failures are almost always caused by the code itself, not by configuration or dependencies — do not modify build config, dependency versions, or test infrastructure to make tests pass.

Never suppress, silence, or bypass a failure. This means: no @Suppress, no @SuppressWarnings, no noinspection comments, no detekt/lint baseline changes, no annotation-based opt-outs, no wrapping code to dodge the checker. If a linter says a suspend modifier is redundant, remove it. If a checker flags an unsafe pattern, rewrite the code. Always address what the tool is actually telling you.

After every fix, re-run the full local check suite to verify the fix works and nothing else regressed. Do not stop until local checks pass. If failures persist after 3 fix attempts, stop and report the remaining failures with your diagnosis.

$ARGUMENTS
