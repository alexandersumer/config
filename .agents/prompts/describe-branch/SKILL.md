---
name: describe-branch
description: Generate an excellent PR description (3 sentences + Conventional Commits summary line)
---

Determine the base branch (prefer the merge-base against `main`/`master`/`develop` or the explicit upstream). Get the cumulative branch diff with three-dot syntax: `git diff base...HEAD`, and inspect `git log base..HEAD --oneline` for additional intent signals. Skim file paths first to identify the primary components touched, then read the meaningful hunks; ignore generated files, lockfiles, and pure formatting noise.

Write a PR description that is sharp, reviewer-friendly, and grounded only in the diff. Cover the primary components modified — do not enumerate every file.

Output format (exactly this shape, nothing else):

```
<sentence 1: what changed — concrete, in plain language>
<sentence 2: why it changed or the user/reviewer impact>
<sentence 3: notable constraint, risk, follow-up, or explicit scope boundary>

<type>[optional scope]: <description>
```

Acceptance criteria for the prose:
- Exactly 3 sentences, each on its own line, no bullets, lists, headings, or code fences.
- Written in the present tense, active voice, and from the reader's perspective.
- Specific and falsifiable: name the component, behavior, or contract that changed; avoid vague phrases like "various improvements", "refactor code", or "update files".
- No restating the diff line-by-line and no internal implementation trivia that isn't user- or reviewer-facing.
- No marketing language, no emojis, no issue keys unless they appear in the branch/commits.

Acceptance criteria for the summary line (must follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/)):
- Structure: `<type>[optional scope][!]: <description>`.
- `type` is one of: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Pick the type that reflects the dominant user-visible change in the diff (prefer `feat`/`fix` over `chore`/`refactor` when applicable).
- `scope` is optional, lowercase, a single short noun in parentheses identifying the affected area (e.g. `auth`, `api`, `parser`); omit it if no single scope dominates.
- Use `!` before the colon when the change is breaking (and ensure sentence 3 calls out the break).
- `description` is lowercase, imperative mood ("add", not "adds"/"added"), no trailing period, ≤72 characters total including type/scope, and reads as a valid squash-merge subject.
- Must be consistent with the 3-sentence prose above (same change, same scope).

Selection guidance:
- If the diff adds new user-visible capability → `feat`.
- If it corrects incorrect behavior → `fix`.
- If it only restructures code with no behavior change → `refactor`.
- If it only changes docs/tests/build/CI → `docs`/`test`/`build`/`ci` respectively.
- If multiple types apply, choose the one matching the highest-impact change and mention the secondary change in sentence 1 or 3.
