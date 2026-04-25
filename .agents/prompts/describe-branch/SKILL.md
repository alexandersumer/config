---
name: describe-branch
description: Generate a Conventional Commits PR title and description
---

Determine the base branch from the explicit upstream or remote default branch, falling back to common default branch names only when necessary. Get the cumulative branch diff with three-dot syntax: `git diff base...HEAD`, and inspect `git log base..HEAD --oneline` for additional intent signals. Skim file paths first to identify the primary components touched, then read the meaningful hunks; ignore generated files, lockfiles, and pure formatting noise.

Write a PR title and description that follow the [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/) message structure. The first line is the PR title and must be a valid Conventional Commit subject suitable for squash merge. The optional description is the Conventional Commit body and footers: explain what changed and why, call out breaking changes or follow-ups when applicable, and stay grounded only in the diff.

Output format (exactly this shape, nothing else):

```
<type>[optional scope][!]: <description>

[optional body]

[optional footer(s)]
```

Acceptance criteria for the subject line:
- Structure: `<type>[optional scope][!]: <description>`.
- `type` is one of: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Pick the type that reflects the dominant user-visible change in the diff (prefer `feat`/`fix` over `chore`/`refactor` when applicable).
- `scope` is optional, lowercase, a single short noun in parentheses identifying the affected area (e.g. `auth`, `api`, `parser`); omit it if no single scope dominates.
- Use `!` before the colon when the change is breaking and include a `BREAKING CHANGE:` footer.
- `description` is lowercase, imperative mood ("add", not "adds"/"added"), no trailing period, ≤72 characters total including type/scope, and reads as a valid squash-merge subject.

Acceptance criteria for the body and footers:
- Body is optional; include it when the subject alone is not enough for a reviewer to understand what changed and why.
- Body paragraphs explain what changed and why, not implementation minutiae. Wrap lines around 72 characters.
- Do not force a fixed number of sentences, bullets, or sections; use the Conventional Commits body format.
- Footers are optional Git-trailer-style lines such as `Refs: ABC-123`, `Co-authored-by: ...`, or `BREAKING CHANGE: ...`.
- If the subject uses `!`, include a `BREAKING CHANGE:` footer describing the break.
- No marketing language, no emojis, no issue keys unless they appear in the branch/commits.

Type selection: `feat` for new user-visible capability, `fix` for incorrect behavior, `refactor` for restructuring with no behavior change, otherwise `docs`/`test`/`build`/`ci`. If multiple apply, pick the highest-impact one and mention secondary changes in the body.
