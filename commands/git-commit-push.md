---
name: git-commit-push
description: Stage all changes, commit with auto-generated message, and push
---

Stop if working tree is clean.

Stage all changes, commit, push. If push to a protected branch is rejected, create a short descriptive branch from the current commit and push that instead — never ask, just do it.

Commit message: single sentence, sentence case, no punctuation, no emojis, no conventional-commit prefixes.

After pushing, if not on the default branch, output the existing PR URL or the create-PR URL.
