---
name: describe-diff
description: Summarize working changes
---

If the working tree is clean, output `clean`.

Otherwise inspect staged and unstaged changes and output one phrase:
- at most 10 words
- no punctuation
- primary change only, not file inventory

Examples:
- `add retry to http client`
- `rename user service to account service`
- `fix off-by-one in pagination cursor`
