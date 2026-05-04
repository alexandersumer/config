---
name: describe-branch
description: Generate a Conventional Commits pull request title and description
---

Generate the canonical pull request title and description for the current branch.

Do not satisfy this with the branch name, issue title, provider default, or a vague summary. The known failure mode is a non-Conventional-Commit title that looks human-friendly but breaks squash-merge history. The first line must be a valid Conventional Commit subject and must be usable verbatim as the PR title.

Determine the base branch from explicit upstream or remote default, falling back to common default names only when necessary. Inspect:
- `git diff base...HEAD`
- `git log base..HEAD --oneline`
- meaningful hunks after skimming paths

Ignore generated files, lockfiles, and pure formatting noise unless they are the change.

Output exactly:
```text
<type>[optional scope][!]: <description>

[optional body]

[optional footer(s)]
```

Subject rules:
- Must match exactly: `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`.
- Type must be one of `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.
- Pick the dominant user-visible type; prefer `feat`/`fix` over `chore`/`refactor` when applicable.
- Scope is optional, lowercase, one short noun.
- Description is lowercase imperative, no trailing period, total first line at most 72 characters.
- Use `!` only for breaking changes and include a `BREAKING CHANGE:` footer.
- Do not include issue keys, branch fragments, emojis, markdown, labels, quotes, or prefixes before the type.

Body rules:
- Prefer a body for non-trivial diffs.
- Explain what changed, why, and the diff-backed implementation details a reviewer needs.
- Use plain Conventional Commit paragraphs, not PR-template headings or checklists.
- Do not invent risk, testing, migration, motivation, or follow-up notes unsupported by the diff, commit history, branch name, or explicit context.

Before returning, silently validate the first line against the regex. If it fails, rewrite it.
