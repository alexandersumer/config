---
name: describe-branch
description: Generate a PR description (3 sentences + summary line)
---

Determine the base branch and get the cumulative branch diff using three-dot syntax (`git diff base...HEAD`). Do not use two-dot diff as it includes unrelated changes from the base branch. Write a direct, concise PR description of what changed and why. Limit to 3 sentences. Output each sentence on its own line. Mention only the primary components modified, not internal implementation details. Do not use bullet points or lists. End with a summary line under 10 words with no punctuation.
