---
description: Review for bugs, security, architecture, and test coverage (no style nits)
argument-hint: "[optional: focus area]"
---

Analyze the cumulative diff of this branch against its base branch.

If `$ARGUMENTS` is provided, narrow the review to only the parts of the diff relevant to that input. Otherwise, review the entire diff.

Scrutinize for critical bugs, security vulnerabilities, architectural flaws, inelegant design, and missing test coverage. Ignore style nits. Only flag problems with high certainty. Output findings as a list in the format `filename:line_number problem_description problem_fix`. If no issues are found, say so briefly.
