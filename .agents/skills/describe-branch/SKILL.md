---
name: describe-branch
description: Write PR title and description
---

Write the canonical PR title and description for the current branch.

Do not use the branch name, issue title, provider default, or a vague summary. First line must be a valid Conventional Commit subject usable verbatim as the PR title.

Inspect base branch, `git diff base...HEAD`, `git log base..HEAD --oneline`, and meaningful hunks. Ignore generated files, lockfiles, and formatting noise unless they are the change.

Output exactly:
```text
<type>[optional scope][!]: <description>

[optional body]

[optional footer(s)]
```

Subject regex:
`^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`

Rules: lowercase imperative description, no trailing period, no prefixes/emojis/issue keys unless footer-supported, `!` only with `BREAKING CHANGE:` footer.

Body: plain Conventional Commit paragraphs explaining what changed and why, grounded only in diff/history/context. No headings, checklist, invented risk, or invented testing notes.
