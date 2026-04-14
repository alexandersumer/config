---
name: git-commit-push
description: Stage all changes, commit with auto-generated message, and push
---

Stop if working tree is clean.

If the current branch is not the default branch (main/master), stay on it and push to it. Do not create, switch, or rename branches.

If the current branch is the default branch, fetch origin and create a short descriptive branch from `origin/main` (or `origin/master`) before committing. Never ask, just do it.

Stage all changes. Review the staged diff before writing the commit message. Commit and push to the current branch. If that push is rejected because the branch is protected or otherwise disallows the push, create a short descriptive branch from the current commit and push that instead. Never ask, just do it.

Commit message: single sentence, 5-12 words, sentence case, no punctuation, no emojis. No prefixes of any kind: no conventional commit prefixes (feat:, fix:, chore:), no username or branch-name prefixes (jsmith/), no ticket IDs, no category tags. Do not imitate prefixes seen in git log. Just a plain sentence describing the change. Focus on what changed and why, not how.

After pushing, if not on the default branch, output the existing PR URL or the create-PR URL.
