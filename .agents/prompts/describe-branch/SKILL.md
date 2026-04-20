---
name: describe-branch
description: Generate a PR description (3 sentences + summary line)
---

Determine the base branch and get the cumulative branch diff using three-dot syntax (`git diff base...HEAD`). Do not use two-dot diff as it includes unrelated changes from the base branch. Write a direct, concise PR description of what changed and why. Limit to 3 sentences. Output each sentence on its own line. Mention only the primary components modified, not internal implementation details. Do not use bullet points or lists. End with a [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/) summary line in the form `<type>[optional scope]: <description>` (lowercase, imperative, no trailing period, ≤72 chars). Suitable for use as a squash-merge commit subject.
