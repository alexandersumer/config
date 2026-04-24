---
name: git-commit-push
description: Stage all changes, commit with auto-generated Conventional Commits message, and push
---

If the working tree is clean, stop.

Branch selection (do not ask, just act):
- On a non-default branch: stay on it. Do not create, switch, or rename.
- On the default branch (`main`/`master`): `git fetch origin`, then create a short descriptive branch from `origin/<default>` and switch to it before committing.

Stage all changes. Read the staged diff. Write the commit message. Commit. Push to the current branch. If that push is rejected (protected branch or otherwise), create a short descriptive branch from the current commit and push that instead.

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
- Subject matches `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`.
- No emojis, no ticket IDs in the subject, no imitation of non-conforming prefixes seen in `git log`.
- After pushing from a non-default branch, output the existing PR URL or the create-PR URL.

Examples: `docs: correct spelling of CHANGELOG`, `feat(lang): add Polish language`, `feat(api)!: drop support for legacy clients`.
