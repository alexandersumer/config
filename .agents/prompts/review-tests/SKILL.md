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

Determine the base branch. Get the cumulative branch diff with three-dot syntax: `git diff base...HEAD`. Read the production code under test before evaluating its tests. Narrow the review to `$ARGUMENTS` if provided.

For each test in scope, run a mutation check: imagine a plausible bug in the production code it covers (off-by-one, swapped arguments, null vs empty, flipped conditional, missing await, wrong exception type). If the test would still pass under that mutation, strengthen it until it would fail.

Apply these strengthening moves:
- Replace weak assertions (`assertNotNull`, `assertTrue(x.size() > 0)`) with specific ones on the actual values.
- Rewrite tests that assert on implementation structure (private fields, mock call order with no behavioral meaning) to assert on observable behavior.
- Add edge case coverage that maps to a real failure mode in the code under test.

Out of scope: tests for trivial code (plain getters/setters, generated code, framework boilerplate), framework or style conventions of the existing test suite.

After edits, run the build to verify tests pass.

Acceptance criteria:
- Every strengthened test would catch at least one plausible mutation it previously missed.
- No new tests for trivial code.
- Build is green.
- Final report lists each touched test as `<file>::<test_name> — <what changed and which mutation it now catches>`.
