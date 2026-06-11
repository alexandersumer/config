---
name: git-publish
description: Commit, push, and open a no-reviewer PR for the current changes
---

Commit current staged, unstaged, and relevant untracked changes, push to `origin`, then create or report a no-reviewer pull request to the remote default branch.

Invoking this skill is explicit authorization to perform the required git writes for publishing: stage intended changes, create a commit, create a branch when needed, push, and create the PR. Do not pause to ask for publish permission unless the inspected changes are incoherent, risky, or ambiguous.

Optimize for a smooth, safe publish: one coherent commit subject, no provider-generated titles, no branch-name summaries, no accidental wrong-repo PRs, no duplicate PRs.

Non-default branches are normal publish sources. Never end with “PR not created” merely because the current branch is non-default or because the workflow started from an existing feature branch; after pushing, run the PR flow for that branch.

## Workflow

1. Inspect repository context before any write:
   - `git rev-parse --show-toplevel`
   - `git branch --show-current`
   - remote default branch, usually from `git remote show origin`
   - `git status --short`
   - effective publish diff for one subject:
     - staged diff: `git diff --cached`
     - unstaged diff: `git diff`
     - relevant untracked files from `git ls-files --others --exclude-standard`, rendered or summarized as new-file diffs
2. Build the effective changes from both sources:
   - branch diff: committed changes reachable from `HEAD` and not in the remote default branch, including local commits on the default branch before a publish branch is created
   - working-tree diff: staged, unstaged, and relevant untracked changes
3. If there is no branch diff and no working-tree diff, stop.
4. Confirm the combined effective changes form one coherent subject. If the committed branch diff and working-tree diff are unrelated, stop and ask how to split or scope the publish.
5. Choose a valid Conventional Commit subject from the combined effective changes.
6. If currently on `main`, `master`, or the remote default branch, create a focused branch from the current `HEAD` before staging or committing, named from the subject, for example `fix/short-topic`; do not push local default-branch commits directly to the default branch.
7. If there are publishable working-tree changes, stage intended changes unless explicitly excluded, then commit them with the chosen subject before pushing. Do not stage unrelated files.
8. If there are no publishable working-tree changes but the source branch already has a branch diff, skip committing and continue.
9. Push the source branch to `origin`, setting upstream if needed.
10. Always create or report a no-reviewer PR from the source branch to the remote default branch using the PR flow below. This applies equally when starting from an existing non-default branch.

Do not run tests/builds unless explicitly asked.

## Subject rules

Subject regex:

```text
^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$
```

Rules:
- Use a grounded Conventional Commit subject from the actual diff.
- No branch names, issue-title summaries, issue-key prefixes, chat summaries, provider defaults, or trailing period.
- Keep the subject human-readable and specific.

## PR flow

Determine the PR provider from `git remote get-url origin` before choosing tools. Use Bitbucket flows only for Bitbucket-compatible repositories. Use GitHub CLI only for GitHub repositories. If the provider is unsupported or required tooling is unavailable, stop after the push and report the exact PR blocker instead of trying a wrong-provider command.

Never use Bitbucket MCP to create, list, inspect, or update pull requests during publish. Do not add reviewers unless the user explicitly requested reviewers.

### Bitbucket canonical-tool preferred path, when available

Use this path when the current harness exposes canonical PR metadata tools such as `inspect_pr_context`, `save_pr_metadata`, and `ensure_bitbucket_pr`.

1. Determine the target repository root from shell git, not from assumptions:

   ```bash
   git rev-parse --show-toplevel
   ```

2. Inspect canonical PR context for that root after the source branch is pushed. Pass `workspaceRoot` when the tool supports it. The reported git root, source branch, and default branch must match the repository being published.
3. Save fresh PR metadata using:
   - the same Conventional Commit subject as the PR title
   - a grounded body describing what changed and why
   - the exact diff fingerprint returned by the inspection step
4. Ensure the PR through `ensure_bitbucket_pr` with explicit source and destination branches and no reviewers unless reviewers were explicitly requested.
5. If the tool reports an existing PR for the branch, report that PR and stop. Do not create a duplicate.
6. If the tool reports stale/wrong-repo metadata, regenerate metadata for the shell-reported repository root and retry once. If it still fails, stop with the exact blocker.

### Bitbucket CLI fallback path: no-reviewer PR

Use this path only when canonical PR tools are unavailable or the repository/tooling is not compatible with the current harness's PR tools.

1. Check once for an existing open PR:

   ```bash
   twg bb prs query --source <branch> --dest <default-branch> -n 5
   ```

   Treat empty output, `[]`, `No pull requests found.`, or output containing `Found 0 pull requests` as no existing PR.

2. If no PR exists, create one with no reviewers:

   ```bash
   twg bb prs create --title "<subject>" --source <branch> --dest <default-branch>
   ```

   Add a description only when a grounded PR body is available.

3. Do not pass reviewer flags. Create PRs with no reviewers unless the user explicitly requested reviewers.
4. If `twg` is unavailable, use `bb pr create` only through its interactive flow and select `Skip (no reviewers)`. Do not use fully specified non-interactive `bb pr create`, because it may apply default reviewers.
5. If PR creation fails:
   - check once for an existing branch PR with CLI
   - if found, report it and stop
   - otherwise retry once with the no-reviewer `twg` Bitbucket create path
   - if it still fails, stop and report the exact blocker

### GitHub CLI fallback path

Use this path only when `origin` is a GitHub repository and `gh` is available.

1. Check once for an existing open PR:

   ```bash
   gh pr list --head <branch> --base <default-branch> --state open --json url --limit 5
   ```

2. If no PR exists, create one without reviewers:

   ```bash
   gh pr create --title "<subject>" --body "<body>" --base <default-branch> --head <branch>
   ```

   Use a grounded body. If no body is available, use the subject as a minimal body rather than provider defaults.

3. Do not pass reviewer flags unless the user explicitly requested reviewers.
4. If `gh` is unavailable or PR creation fails, check once for an existing branch PR with `gh pr list`; if none is found, stop and report the exact blocker.

## Final response

Report:
- commit hash, or state that no new commit was needed
- subject / PR title
- branch
- push result
- PR result
