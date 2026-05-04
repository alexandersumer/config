---
name: review-tests
description: Strengthen branch tests
argument-hint: "[optional: focus area]"
inputs:
  - name: focus_area
    label: Focus area
    description: Optional text to narrow the review to specific tests or areas. Leave empty to review all tests in the branch.
    type: string
    required: false
---

Review tests for `git diff base...HEAD`, narrowed by `$ARGUMENTS` if provided. Read production code first.

Do not add coverage theater. A useful test catches a realistic bug: flipped branch, off-by-one, swapped argument, null vs empty, missing await, wrong exception, stale cache, auth bypass, schema drift.

Strengthen only important changed behavior:
- replace weak assertions with exact observable outcomes
- prefer public behavior over private fields or mock call order
- add edge/failure cases tied to real code paths

Skip trivial getters, generated code, framework boilerplate, and style conventions.

Run targeted tests and the build when available.

Final, one per touched test:
`<file>::<test_name> — catches <named bug>`

If none: `no test changes justified`
