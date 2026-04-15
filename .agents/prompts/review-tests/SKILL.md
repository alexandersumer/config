---
name: review-tests
description: Review and strengthen tests in a branch for depth, mutation resilience, and correctness
argument-hint: "[optional: focus area]"
inputs:
  - name: focus_area
    label: Focus area
    description: Optional text to narrow the review to specific tests or areas. Leave empty to review all tests in the branch.
    type: string
    required: false
---

Determine the base branch and get the cumulative branch diff using three-dot syntax (`git diff base...HEAD`). Read the production code under test before evaluating tests. If `$ARGUMENTS` is provided, narrow the review to that area.

For each test, mentally introduce a plausible bug in the production code (off-by-one, swapped arguments, null instead of empty, flipped conditional). If the test would not catch it, strengthen it so it would. Add missing edge case coverage that matters. Replace weak assertions with specific ones. Rewrite tests that mirror implementation structure to verify observable behavior instead.

Do not add tests for trivial code. Do not touch style or framework conventions. After all changes, run the build to verify tests pass. Report a brief summary of what was strengthened and why.
