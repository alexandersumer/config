---
name: describe-branch
description: Generate a Conventional Commits pull request title and description
---

Determine the base branch from the explicit upstream or remote default branch, falling back to common default branch names only when necessary. Get the cumulative branch diff with three-dot syntax: `git diff base...HEAD`, and inspect `git log base..HEAD --oneline` for additional intent signals. Skim file paths first to identify the primary components touched, then read the meaningful hunks; ignore generated files, lockfiles, and pure formatting noise.

Write a pull request title and description that follow the [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/) message structure. The first line is the pull request title and must be a valid Conventional Commit subject suitable for squash merge. The description is the canonical Conventional Commit body and footers: explain what changed, why it changed, and how the diff supports that interpretation; call out breaking changes or follow-ups when applicable; and stay grounded only in the diff.

This output is the canonical pull request title and description. Any caller or SCM/code-review tool that uses this output must use the first line verbatim as the pull request title and the rest as the pull request description; do not substitute the branch name, a prose summary, an issue-key prefix, a sentence-case summary, or a tool-default title. Favor a real Conventional Commit body over a terse two-line summary: omit the body only for truly trivial, single-purpose diffs where the subject fully explains the change. If you cannot produce a first line that passes the Conventional Commit subject regex below, revise it until it does before returning output.

Output format (exactly this shape, nothing else):

```
<type>[optional scope][!]: <description>

[optional body]

[optional footer(s)]
```

Acceptance criteria for the subject line:
- Structure: `<type>[optional scope][!]: <description>`.
- The complete first line must match this regex exactly: `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`.
- `type` is one of: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Pick the type that reflects the dominant user-visible change in the diff (prefer `feat`/`fix` over `chore`/`refactor` when applicable).
- `scope` is optional, lowercase, a single short noun in parentheses identifying the affected area (e.g. `auth`, `api`, `parser`); omit it if no single scope dominates.
- Use `!` before the colon when the change is breaking and include a `BREAKING CHANGE:` footer.
- `description` is lowercase, imperative mood ("add", not "adds"/"added"), no trailing period, ≤72 characters total including type/scope, and reads as a valid squash-merge subject.
- Do not include issue keys, branch-name fragments, ticket prefixes, emojis, markdown, quotes, or labels before the Conventional Commit type.
- Do not imitate non-conforming commit prefixes found in `git log`; normalize the title to the Conventional Commit structure above.

Acceptance criteria for the body and footers:
- Prefer including a body. Omit it only when the diff is trivial and the subject fully explains both the change and its reason.
- Body must use canonical Conventional Commit prose: one or more plain paragraphs after the blank line following the subject, not markdown headings, checklists, or pull request template sections.
- For non-trivial diffs, include enough body detail for a reviewer to understand the branch without reading every hunk: summarize the affected behavior, explain why the change is needed, and describe important diff-backed supporting changes.
- Use multiple paragraphs when the branch contains distinct behavior changes or separate affected areas. Do not collapse unrelated changes into a single vague sentence.
- Body paragraphs explain what changed and why, with only the implementation detail needed to make the behavior understandable. Wrap lines around 72 characters.
- Do not invent risk, testing, migration notes, follow-ups, motivations, or consequences that are not directly supported by the diff, commit history, branch name, or explicit user context.
- Do not use markdown headings, forced pull request sections, marketing language, emojis, or issue keys unless issue keys appear in the branch/commits.
- Footers are optional Git-trailer-style lines such as `Refs: ABC-123`, `Co-authored-by: ...`, or `BREAKING CHANGE: ...`.
- If the subject uses `!`, include a `BREAKING CHANGE:` footer describing the break.

Type selection: `feat` for new user-visible capability, `fix` for incorrect behavior, `refactor` for restructuring with no behavior change, otherwise `docs`/`test`/`build`/`ci`. If multiple apply, pick the highest-impact one and mention secondary changes in the body.

Before returning, silently validate the final answer:
- The very first character of the response starts the Conventional Commit type; there is no preamble.
- The first line matches `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`.
- The first line is not a branch name, issue-key title, sentence-case prose summary, markdown heading, bullet, or quoted string.
- If any check fails, rewrite the title and re-check before outputting.
