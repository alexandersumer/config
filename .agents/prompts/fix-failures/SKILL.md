---
name: fix-failures
description: Run relevant build/tests and fix real failures without suppressions
argument-hint: "[optional: error output or stack trace]"
inputs:
  - name: error_output
    label: Error output or stack trace
    description: Paste error output or stack trace to fix. Leave empty to run the build and test suite to discover failures.
    type: string
    required: false
---

Fix the real failure. Use `error_output`, else `$ARGUMENTS`, else run the repo's local check suite after reading README/CONTRIBUTING/build files.

Do not satisfy this by rerunning the same command until it happens to pass. The known failure mode is treating test output as the task instead of evidence. Every run must change your knowledge, your hypothesis, or the code.

For each failure:
1. Reproduce it locally, using the smallest command that still exercises the failure.
2. Identify the root cause in application code or in a test that asserts intended behavior incorrectly.
3. Fix the root cause. Do not mute the symptom.
4. Re-run the targeted command, then the broader local suite once targeted evidence is green.

Allowed changes:
- application source code
- tests that assert the correct behavior

Forbidden shortcuts:
- warning suppressions, lint baselines, annotation opt-outs
- dependency bumps or build-config edits
- test-infra edits, skipped tests, deleted tests, weakened assertions
- wrapper code that dodges the checker instead of fixing the defect

If the same command fails three times without new information, stop rerunning it. Build a smaller reproducer, inspect the boundary it exercises, or report the blocker with diagnosis.

Final response:
- Fixed: `<root cause>`
- Changed: `<files>`
- Targeted check: `<command>` -> `<result>`
- Full check: `<command>` -> `<result or not run: reason>`
- Remaining: `<none or diagnosed blockers>`
