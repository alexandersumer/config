---
name: describe-diff
description: Generate a ≤10 word phrase summarizing working changes
---

If the working tree is clean, output exactly `clean` and stop.

Otherwise inspect staged and unstaged changes and output one phrase naming the primary modification.

Do not satisfy this by listing files. The known failure mode is a vague inventory instead of the change's intent.

Output rules:
- at most 10 words
- no punctuation
- names the primary change, not every touched file

Examples:
- `add retry to http client`
- `rename user service to account service`
- `fix off-by-one in pagination cursor`
