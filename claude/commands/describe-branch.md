---
description: Generate a PR description (3 sentences + summary line)
---

Determine the base branch using `git merge-base HEAD main` (or `master` if main doesn't exist). Analyze the cumulative diff of this branch against the base. Write a direct, concise PR description of what changed and why. Limit response to 3 sentences. Mention only the primary components modified, avoiding internal implementation details or minor method names. Do NOT use bullet points or lists. Use full sentences only. End with an overview summary line under 10 words with no punctuation.
