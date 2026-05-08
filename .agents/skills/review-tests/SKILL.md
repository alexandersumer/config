---
name: review-tests
description: Strengthen branch tests
---

Review tests for `git diff base...HEAD`, narrowed by `focus_area` or `$ARGUMENTS` if provided. Read production code first.

Do not add coverage theater. A useful test catches a realistic bug: flipped branch, off-by-one, swapped argument, null vs empty, missing await, wrong exception, stale cache, auth bypass, schema drift.

Strengthen only important changed behavior:
- replace weak assertions with exact observable outcomes
- prefer public behavior over private fields or mock call order
- add edge/failure cases tied to real code paths

Skip trivial getters, generated code, framework boilerplate, and style conventions.

Run targeted tests. Run the build if available. If no check applies, say why.

Final, one per touched test:
`<file>::<test_name> — catches <named bug>`

If none: `no test changes justified`
