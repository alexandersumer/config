---
name: describe-branch
description: Generate a PR description (3 sentences + summary line)
---

Determine the base branch. Get the cumulative branch diff with three-dot syntax: `git diff base...HEAD`.

Write a PR description of what changed and why, covering only the primary components modified.

Output format (exactly this shape):

```
<sentence 1: what changed>
<sentence 2: why or impact>
<sentence 3: notable constraint, follow-up, or scope boundary>

<type>[optional scope]: <description>
```

Acceptance criteria:
- Exactly 3 sentences of prose, each on its own line, no bullets or lists.
- Final line follows [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/): lowercase, imperative, no trailing period, ≤72 chars; suitable as a squash-merge subject.
- No mention of internal implementation details that aren't user- or reviewer-facing.
