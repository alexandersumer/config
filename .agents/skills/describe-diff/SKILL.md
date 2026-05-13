---
name: describe-diff
description: Summarize working changes. Use when the user wants a short diff summary, change phrase, or clean/dirty working-tree description.
register_cmd: true
---

If the working tree is clean, output `clean`.

Otherwise inspect staged, unstaged, and untracked working-tree changes and output one phrase:
- at most 10 words
- no punctuation
- primary change only, not file inventory

Examples:
- `add retry to http client`
- `rename user service to account service`
- `fix off-by-one in pagination cursor`
