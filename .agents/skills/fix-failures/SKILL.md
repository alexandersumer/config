---
name: fix-failures
description: Fix real check failures. Use when tests, builds, linters, CI, or pasted error output fail and the user wants the root cause fixed.
---

Fix the failure from `error_output`, `$ARGUMENTS`, or the repo's normal checks.

Do not keep rerunning the same command. Each run must change the code, the hypothesis, or the next diagnostic step.

Read the local check instructions. Reproduce the failure with the smallest useful command. Fix the root cause in application code or in a test that asserts intended behavior incorrectly.

Forbidden: suppressions, baselines, dependency bumps, build config edits, test infra edits, skipped/deleted/weakened tests, wrapper code that dodges the checker.

Run the targeted check, then the broader suite when targeted evidence is green.

Final:
- Fixed: `<root cause>`
- Changed: `<files>`
- Checks: `<targeted>`; `<broad or not run: reason>`
- Remaining: `<none or blocker>`
