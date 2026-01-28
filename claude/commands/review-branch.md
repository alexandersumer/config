---
description: Strict Senior Engineer review for bugs/security (no style nits)
---

Determine the base branch using `git merge-base HEAD main` (or `master` if main doesn't exist). Analyze the cumulative diff of this branch against the base. Act as a strict Senior Engineer to scrutinize this branch for critical bugs, security vulnerabilities, architectural flaws, inelegant design, and missing test coverage. Ignore all style nits. Strictly avoid hallucinations by only flagging problems with high certainty. Output findings strictly as a list in the format `filename:line_number problem_description problem_fix`, ensuring each is robust, concise, and clear.
