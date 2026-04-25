---
name: git-commit-push-to-origin
description: Stage changes, commit with a Conventional Commits message, push to origin, and create or report the PR
---

If the working tree is clean, stop.

## Tool preference

Before any source-control operation, detect the repository host from the remote URL and check whether a matching SCM/code-review integration or CLI is available for that host (Bitbucket, GitHub, GitLab, or otherwise). Do not assume Bitbucket; use the provider that matches the current repository.

Host/tool discovery checklist:
- Inspect `git remote -v` to identify the repository host and remote name.
- Identify the current branch and repository default branch.
- Check for a matching SCM/code-review integration or CLI for that host.
- Use the host-matched tool for remote operations and PR discovery/creation when it supports the operation.

Rules:
- Prefer the host-matched SCM/code-review tool when it can perform the operation safely and completely.
- Use direct `git` only when no tool supports the operation, or when the operation is inherently local (for example: inspect the working tree, stage files, create a local commit).
- Do not mix tool and direct-git approaches for the same remote operation unless the tool fails or lacks the required capability.
- When falling back to direct `git`, briefly state why.

## Commit and push flow

Stage all changes. Read the staged diff. Write the commit message. Commit. Push to origin on the current branch.

Use an available SCM tool for push/remote publication if it can perform the operation for the current host and branch. Otherwise, fall back to direct `git push origin <current-branch>`.

## Pull request flow

After a successful push to origin from a non-default branch, do not stop at a provider-generated create-PR URL when an SCM/code-review tool can create or discover pull requests.

1. Use the available host-matched SCM/code-review tool to look for an existing open PR from the pushed branch into the repository's default branch.
2. If an existing PR is found, output its URL and briefly note whether the title is a valid Conventional Commit subject and whether it already has a meaningful description. If the existing title is not a valid Conventional Commit subject and the SCM/code-review tool supports updating PR titles, update the title to the Conventional Commit subject derived from the commit(s); otherwise call out the non-conforming title explicitly. Do not overwrite descriptions or post comments unless the user explicitly requested that.
3. If no PR exists and the SCM/code-review tool supports PR creation, create the PR using:
   - **title**: the Conventional Commit subject from the commit message.
   - **description**: a Conventional Commits body and optional footers grounded in the pushed diff. Explain what changed and why, include `BREAKING CHANGE:` when applicable, and do not force fixed sections. Do not create an empty-description PR when the subject alone is insufficient for review.
   - **source branch**: the pushed origin branch.
   - **target branch**: the repository default branch unless upstream configuration clearly indicates another target.
4. If the SCM/code-review tool cannot create PRs, or PR creation fails for a reason outside your control, explicitly say why and then return the create-PR URL as a fallback.

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
- After pushing to origin from a non-default branch, the final response includes either an existing PR URL or the URL of the PR you created with the host-matched SCM/code-review tool.
- A create-PR URL alone is not acceptable when a tool can create the PR.
- Any PR title (newly created or, when the tool supports updates, also pre-existing) is a valid Conventional Commit subject matching the commit subject regex above; never use the branch name, a prose summary, or a tool-default title. The PR description is a Conventional Commits body with optional footers, non-empty when the subject alone is insufficient for review.

Examples: `docs: correct spelling of CHANGELOG`, `feat(lang): add Polish language`, `feat(api)!: drop support for legacy clients`.
