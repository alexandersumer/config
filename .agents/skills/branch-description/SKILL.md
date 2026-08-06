---
name: branch-description
description: Write a PR or branch title and description. Use when the user wants a pull request title, PR body, branch summary, or Conventional Commit description.
---

Write the canonical PR title and description for the current branch.

This skill is read-only. Never run `git add`, `git commit`, `git push`, `gh`, `bb`, `hub`, or provider commands that create, update, or publish a PR.

Do not use the branch name, issue title, provider default, conversation recency, your own last action, the latest commit, or a vague summary. First line must be a valid Conventional Commit subject usable verbatim as the PR title.

Resolve the comparison base from the remote default branch: use `origin/HEAD`, falling back to `origin/main` or `origin/master` only if needed, then set `<base>` to `git merge-base <remote-default> HEAD`. If no remote default or merge-base exists, omit only the committed-branch part and still inspect the working tree.

Build one effective change set from every available source: `git log <base>..HEAD --oneline` and `git diff <base>..HEAD` when `<base>` exists, staged changes with `git diff --cached`, unstaged changes with `git diff`, and untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs. Always include staged, unstaged, and untracked changes even when the committed branch diff exists. If the committed branch diff is empty but the working tree has staged, unstaged, or untracked changes, describe those changes instead of treating the branch as empty. Ignore generated files, lockfiles, and formatting noise unless they are the change.

When the effective change set is empty, output exactly this diagnostic sentence and stop, replacing placeholders:
```text
No branch description to write: current branch `<current-branch>` has no changes relative to remote default `<remote-default-ref>`, and there are no staged, unstaged, or untracked changes.
```

Use the current branch from `git branch --show-current`, or `HEAD` if detached. Use the resolved remote default ref from the base step: `origin/main`, `origin/master`, or `origin/<branch>` from `origin/HEAD`. Do not assume `main`. If no remote default or merge-base exists, use this variant instead:
```text
No branch description to write: current branch `<current-branch>` has no resolved remote-default comparison base, and there are no staged, unstaged, or untracked changes.
```

The cumulative effective diff is the source of truth. Use conversation context only to clarify intent that is supported by the diff; never let the most recent agent edit, latest local fix, or last commit dominate the title unless it is the primary branch-level change.

Before writing, identify the highest-level user-visible or system-behavior change that explains most meaningful diffs. If the diff is implementation-heavy, read enough changed code, callers, or tests to infer the observable what and why. The subject should name the branch-level intent. Logging, tests, cleanup, refactors, and follow-up fixes belong in the body only when they support the main change, unless they are the actual purpose of the branch. If the effective change set contains unrelated changes, do not invent a single purpose; use the safest accurate subject and explain the split plainly in the body.

If the user asks for a "description", produce the Conventional Commit body/PR body in this format, still headed by the validated Conventional Commit subject. Do not answer with a branch activity summary.

For non-empty effective changes, output exactly:
```text
<type>[optional scope][!]: <description>

[optional body]

[optional footer(s)]
```

Subject regex:
`^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`

Rules: lowercase imperative description, no trailing period, no prefixes/emojis/issue keys unless footer-supported, `!` only with `BREAKING CHANGE:` footer.

For non-empty effective changes, do not output commit hashes, branch names, push results, PR URLs, test status, publish status, or labels like `Commit:`, `Subject:`, `Branch:`, or `Push/PR result:`. The empty-change diagnostic above is exempt because it identifies the checked branch and base.

Body: plain Conventional Commit paragraphs explaining what changed overall, why the branch exists, and how notable implementation details support the main change, grounded only in the cumulative diff/history/context. No headings, checklist, invented risk, or invented testing notes.
