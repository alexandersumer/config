---
name: review-tests
description: Review and strengthen branch tests for meaningful coverage and regression resistance
argument-hint: "[optional: focus area]"
inputs:
  - name: focus_area
    label: Focus area
    description: Optional text to narrow the review to specific tests or areas. Leave empty to review all tests in the branch.
    type: string
    required: false
---

Review and strengthen tests for the current branch. Determine the base branch and inspect `git diff base...HEAD`. Read the production code before judging its tests. Narrow to `$ARGUMENTS` only if provided.

Do not satisfy this by adding more assertions that would pass under the same bug. The known failure mode is coverage theater: tests that check existence, mocks, snapshots, or implementation shape without proving behavior.

For each important changed behavior, name a plausible mutation or regression: flipped branch, off-by-one, swapped argument, null vs empty, missing await, wrong exception, stale cache, permission bypass, schema drift. A test is meaningful only if it would catch that bug.

Strengthen tests when useful:
- Replace weak assertions with exact observable outcomes.
- Prefer public-entry behavior over private fields or mock call order.
- Add edge/failure cases tied to real code paths.
- Do not add tests for trivial getters, generated code, framework boilerplate, or style conventions.

After edits, run the targeted test command and the broader build when available.

Final report, one touched test per line:
`<file>::<test_name> — <what changed> — catches: <named mutation>`

If no test should change, output exactly:
`no test changes justified`
