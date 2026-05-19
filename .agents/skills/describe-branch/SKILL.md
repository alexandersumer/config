---
name: describe-branch
description: Write PR title and description. Use when the user wants a pull request title, PR body, branch summary, or Conventional Commit description.
register_cmd: true
---

Write the canonical PR title and description for the current branch.

This skill is read-only. Never run `git add`, `git commit`, `git push`, `gh`, `bb`, `hub`, or provider commands that create, update, or publish a PR.

Do not use the branch name, issue title, provider default, or a vague summary. First line must be a valid Conventional Commit subject usable verbatim as the PR title.

Inspect base branch, `git log base..HEAD --oneline`, and the effective branch/worktree diff: `git diff base...HEAD`, `git diff --cached`, `git diff`, and untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs. If the committed branch diff is empty but the working tree has staged, unstaged, or untracked changes, describe those changes instead of treating the branch as empty. Ignore generated files, lockfiles, and formatting noise unless they are the change.

If the user asks for a "description", produce the Conventional Commit body/PR body in this format, still headed by the validated Conventional Commit subject. Do not answer with a branch activity summary.

Output exactly:
```text
<type>[optional scope][!]: <description>

[optional body]

[optional footer(s)]
```

Subject regex:
`^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`

Rules: lowercase imperative description, no trailing period, no prefixes/emojis/issue keys unless footer-supported, `!` only with `BREAKING CHANGE:` footer.

Do not output commit hashes, branch names, push results, PR URLs, test status, publish status, or labels like `Commit:`, `Subject:`, `Branch:`, or `Push/PR result:`.

Body: plain Conventional Commit paragraphs explaining what changed and why, grounded only in diff/history/context. No headings, checklist, invented risk, or invented testing notes.
