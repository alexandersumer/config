---
name: git-commit-push-to-origin
description: Stage all changes, commit with auto-generated Conventional Commits message, and push directly to origin, preferring available SCM tools
---

If the working tree is clean, stop.

## Tool preference

Before any source-control operation, check whether an SCM integration/tool is available for this repository host (Bitbucket, GitHub, GitLab, or otherwise).

- Prefer the SCM tool when it can perform the operation safely and completely.
- Use direct `git` only when no tool supports the operation, or when the operation is inherently local (for example: inspect the working tree, stage files, create a local commit).
- Do not mix tool and direct-git approaches for the same remote operation unless the tool fails or lacks the required capability.
- When falling back to direct `git`, briefly state why.

## Commit and push flow

Stage all changes. Read the staged diff. Write the commit message. Commit. Push to origin on the current branch.

Use an available SCM tool for push/remote publication if it can perform the operation for the current host and branch. Otherwise, fall back to direct `git push origin <current-branch>`.

## Commit message

Follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/):

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

- **type**: one of `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Pick the most user-impactful one; split commits if multiple apply.
- **scope** (optional): short lowercase noun in parens, e.g. `feat(parser):`.
- **description**: lowercase, imperative ("add" not "added"), no trailing period, ≤72 chars.
- **body** (optional): blank line after subject. Explain what and why, not how. Wrap ~72 chars.
- **footers** (optional): blank line after body. Git-trailer style: `Refs: ABC-123`, `Co-authored-by: …`. Tokens use `-` for spaces (except `BREAKING CHANGE`).
- **breaking change**: append `!` after type/scope and/or add `BREAKING CHANGE: <desc>` footer.

Acceptance criteria:
- Subject matches the regex `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`.
- No emojis, no ticket IDs in the subject, no imitation of non-conforming prefixes seen in `git log`.

Examples: `docs: correct spelling of CHANGELOG`, `feat(lang): add Polish language`, `feat(api)!: drop support for legacy clients`.
