---
name: describe-diff
description: Generate a ≤10 word phrase summarizing working changes
---

If the working tree is clean, output `clean` and stop.

Otherwise, analyze the current working changes (staged and unstaged) and output a single phrase naming the primary modification.

Acceptance criteria:
- ≤10 words.
- No punctuation.
- Names the primary change, not a list of every file touched.

Examples:
- `add retry to http client`
- `rename UserService to AccountService`
- `fix off-by-one in pagination cursor`
