---
name: describe-branch
description: Generate a Conventional Commits summary followed by a 3-sentence PR description
---

Determine the base branch (prefer the merge-base against `main`/`master`/`develop` or the explicit upstream). Get the cumulative branch diff with three-dot syntax: `git diff base...HEAD`, and inspect `git log base..HEAD --oneline` for additional intent signals. Skim file paths first to identify the primary components touched, then read the meaningful hunks; ignore generated files, lockfiles, and pure formatting noise.

Write a PR summary and description that are sharp, reviewer-friendly, and grounded only in the diff. The first line must be a valid Conventional Commit summary suitable for squash merge. The description must explain the same change in plain language and cover the primary components modified — do not enumerate every file.

Output format (exactly this shape, nothing else):

```
<type>[optional scope][!]: <description>

<sentence 1: what changed — concrete, in plain language>
<sentence 2: why it changed or the user/reviewer impact>
<sentence 3: notable constraint, risk, follow-up, or explicit scope boundary>
```

Acceptance criteria for the summary line (must be first and follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/)):
- Structure: `<type>[optional scope][!]: <description>`.
- `type` is one of: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Pick the type that reflects the dominant user-visible change in the diff (prefer `feat`/`fix` over `chore`/`refactor` when applicable).
- `scope` is optional, lowercase, a single short noun in parentheses identifying the affected area (e.g. `auth`, `api`, `parser`); omit it if no single scope dominates.
- Use `!` before the colon when the change is breaking, and ensure sentence 3 calls out the break.
- `description` is lowercase, imperative mood ("add", not "adds"/"added"), no trailing period, ≤72 characters total including type/scope, and reads as a valid squash-merge subject.
- Must be consistent with the 3-sentence prose below it (same change, same scope).

Acceptance criteria for the description:
- Exactly 3 sentences after the summary, each on its own line, no bullets, lists, headings, or code fences.
- Written in the present tense, active voice, and from the reader's perspective.
- Specific and falsifiable: name the component, behavior, or contract that changed; avoid vague phrases like "various improvements", "refactor code", or "update files".
- No restating the diff line-by-line and no internal implementation trivia that isn't user- or reviewer-facing.
- No marketing language, no emojis, no issue keys unless they appear in the branch/commits.

Type selection: `feat` for new user-visible capability, `fix` for incorrect behavior, `refactor` for restructuring with no behavior change, otherwise `docs`/`test`/`build`/`ci`. If multiple apply, pick the highest-impact one and mention the secondary in sentence 1 or 3.
