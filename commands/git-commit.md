---
name: git-commit
description: Stage all changes and commit with auto-generated message
---

Stop if the working tree is clean. Never commit directly on the default branch (main/master). If currently on the default branch, create a short descriptive branch and switch to it before committing — never ask, just do it. Stage all changes. Review the staged diff before writing the commit message.

Commit message: single sentence, 5–12 words, sentence case, no punctuation, no emojis. No prefixes of any kind — no conventional commit prefixes (feat:, fix:, chore:), no username or branch-name prefixes (jsmith/), no ticket IDs, no category tags. Do not imitate prefixes seen in git log. Just a plain sentence describing the change. Focus on what changed and why, not how.
