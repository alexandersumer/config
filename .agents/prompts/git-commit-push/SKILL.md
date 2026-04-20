---
name: git-commit-push
description: Stage all changes, commit with auto-generated Conventional Commits message, and push
---

Stop if working tree is clean.

If the current branch is not the default branch (main/master), stay on it and push to it. Do not create, switch, or rename branches.

If the current branch is the default branch, fetch origin and create a short descriptive branch from `origin/main` (or `origin/master`) before committing. Never ask, just do it.

Stage all changes. Review the staged diff before writing the commit message. Commit and push to the current branch. If that push is rejected because the branch is protected or otherwise disallows the push, create a short descriptive branch from the current commit and push that instead. Never ask, just do it.

## Commit message

Follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/):

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

- **type**: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Pick the most user-impactful one; split commits if multiple apply.
- **scope** (optional): short lowercase noun in parens, e.g. `feat(parser):`.
- **description**: lowercase, imperative ("add" not "added"), no trailing period. Subject line ≤72 chars.
- **body** (optional): blank line after subject. Explain what and why, not how. Wrap ~72 chars.
- **footers** (optional): blank line after body. Git-trailer style: `Refs: ABC-123`, `Co-authored-by: …`. Tokens use `-` for spaces (except `BREAKING CHANGE`).
- **breaking change**: append `!` after type/scope and/or add `BREAKING CHANGE: <desc>` footer.
- No emojis. No ticket IDs in the subject. Don't imitate non-conforming prefixes from `git log`.

Examples: `docs: correct spelling of CHANGELOG`, `feat(lang): add Polish language`, `feat(api)!: drop support for Node 6`.

After pushing, if not on the default branch, output the existing PR URL or the create-PR URL.
