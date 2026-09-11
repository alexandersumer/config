---
name: git-publish
description: Commit, push, and open a no-reviewer PR for the current changes
---

Commit current staged, unstaged, and relevant untracked changes, push to `origin`, then create or report a no-reviewer pull request to the resolved PR destination branch.

Invoking this skill is explicit authorization to perform the required git writes for publishing: stage intended changes, create a commit, create a branch when needed, push, and create the PR. Do not pause to ask for publish permission unless the inspected changes are incoherent, risky, or ambiguous.

Optimize for a smooth, safe publish: one coherent commit subject, no provider-generated titles, no branch-name summaries, no accidental wrong-repo PRs, no duplicate PRs.

Non-default branches are normal publish sources. Never end with “PR not created” merely because the current branch is non-default or because the workflow started from an existing feature branch; after pushing, run the PR flow for that branch.

## Workflow

1. Inspect repository context before any write:
   - `git rev-parse --show-toplevel`
   - `git branch --show-current`
   - `git rev-parse --verify HEAD`
   - push URLs for provider and push-target decisions: `git remote get-url --push --all origin`
   - local default-branch evidence for the local-first default branch resolver:
     - `refs/remotes/origin/HEAD`, only when it points to an existing local remote-tracking ref
     - local `refs/remotes/origin/main` and `refs/remotes/origin/master`
   - `git status --short`
   - effective publish diff for one subject:
     - staged diff: `git diff --cached`
     - unstaged diff: `git diff`
     - relevant untracked files from `git ls-files --others --exclude-standard`, rendered or summarized as new-file diffs
2. Stop before any write if `HEAD` is detached or missing, the current branch name is empty, `origin` has zero or multiple push URLs, or repository/remote identity is otherwise ambiguous. Git pushes to every configured push URL, so one URL is required for a single safe publish target.
3. Resolve the PR destination branch with the local-first default branch resolver:
   - Use `refs/remotes/origin/HEAD` only when it resolves to an existing local `refs/remotes/origin/<branch>` ref.
   - Otherwise use a local `origin/main` or `origin/master` candidate only when exactly one exists.
   - Treat missing refs, dangling/stale `origin/HEAD`, both `origin/main` and `origin/master` without a valid `origin/HEAD`, or any other conflicting local default evidence as ambiguous.
   - Run live remote discovery only when a PR destination is required and local refs are missing, stale, or conflicting. If live discovery still cannot identify one destination branch, stop before PR creation and report the exact PR destination blocker rather than guessing.
4. Determine the PR provider from the sole push URL. Use that URL for push/provider decisions. If fetch and push URLs point at incompatible providers or repositories such that the review target cannot be identified safely, stop before PR creation and report the exact blocker.
5. Build the effective changes from both sources:
   - branch diff: committed changes reachable from `HEAD` and not in the resolved destination branch, including local commits on the default branch before a publish branch is created
   - working-tree diff: staged, unstaged, and relevant untracked changes
6. If there is no branch diff and no working-tree diff, stop.
7. Confirm the combined effective changes form one coherent subject. If the committed branch diff and working-tree diff are unrelated, stop and ask how to split or scope the publish.
8. Choose a valid Conventional Commit subject and write a non-empty, grounded PR body from the combined effective changes. Use plain paragraphs explaining the overall change and only the why or material implementation details supported by the evidence; do not add headings, checklists, invented issue context, risks, or test results. Freeze this subject/body pair for every provider path below.
9. If currently on `main`, `master`, or the resolved PR destination branch, create a focused branch from the current `HEAD` before staging or committing, named from the subject, for example `fix/short-topic`; do not push local default-branch commits directly to the destination branch.
10. If there are publishable working-tree changes, stage intended changes unless explicitly excluded, then commit them with the chosen subject before pushing. Do not stage unrelated files.
11. If there are no publishable working-tree changes but the source branch already has a branch diff, skip committing and continue.
12. Push the source branch to `origin`, setting upstream if needed.
13. After the source branch has been pushed, always create or report a no-reviewer PR from the source branch to the resolved destination branch using the PR flow below. This applies equally when starting from an existing non-default branch.

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

Determine the PR provider from the sole URL returned by `git remote get-url --push --all origin` before choosing tools. Use Bitbucket flows only for Bitbucket-compatible repositories. Use GitHub CLI only for GitHub repositories. If the provider is unsupported, fetch/push repository identity is unsafe for review targeting, or required tooling is unavailable, stop after the push and report the exact PR blocker instead of trying a wrong-provider command.

Never use Bitbucket MCP to create, list, inspect, or update pull requests during publish. Do not add reviewers unless the user explicitly requested reviewers.
Do not create a new PR with a missing or empty body.

### Provider-compatible managed PR path, when available

Use this path when the current harness exposes a controlled PR facility compatible with the detected provider. Use it only when it can inspect repository identity and existing branch PRs, carry fresh title/body metadata when required, and create or idempotently ensure a PR with explicit source and destination branches. Otherwise use the provider CLI fallback below.

1. Determine the target repository root from shell git, not from assumptions:

   ```bash
   git rev-parse --show-toplevel
   ```

2. Inspect PR context for that root after the source branch is pushed. Pass the workspace root when the facility supports it. The reported git root, provider, source branch, and destination branch must match the repository being published.
3. Supply fresh PR metadata using:
   - the same Conventional Commit subject as the PR title
   - a grounded body describing what changed and why
   - the exact diff fingerprint returned by the inspection step
4. Create or idempotently ensure the PR with explicit source and destination branches and no reviewers unless reviewers were explicitly requested.
5. If the tool reports an existing PR for the branch, report that PR and stop. Do not create a duplicate.
6. If the tool reports stale/wrong-repo metadata, regenerate metadata for the shell-reported repository root and retry once. If it still fails, stop with the exact blocker.

### Bitbucket CLI fallback path: no-reviewer PR

Use the verified push workspace/repository explicitly on every call. Prefer `twg`; if it is unavailable, use `bb` after inspecting its installed help. Do not switch providers or rely on remote auto-detection.

With TWG:

```bash
twg bb prs query --workspace <workspace> --repo <repo> --source <branch> --dest <destination-branch> --state OPEN -n 5 -o json
```

Read the inline/compact records or saved JSON array. A successful query with zero matching rows proves absence; empty stdout, errors, or inaccessible output do not. Report an existing matching PR instead of creating another.

If absent, write the frozen body with real newlines to a private temporary file and create:

```bash
twg bb prs create --workspace <workspace> --repo <repo> --title "<subject>" --source <branch> --dest <destination-branch> --description-file <description-file> -o json
```

Omit reviewer flags unless requested. Check the returned PR description and reviewers. If creation succeeded but the description is blank or missing, update that PR once with `twg bb prs update --workspace <workspace> --repo <repo> --pull-request <id> --description-file <description-file>`, then verify with `twg bb prs get <id> --workspace <workspace> --repo <repo> --full -o json`. Never issue another create to repair metadata. Report an unexpected reviewer or failed repair explicitly rather than claiming a no-reviewer success.

If creation fails without returning a PR, query that same source/destination once. Report a recovered PR after checking its description. Retry create once only if the successful lookup proves absence; otherwise report the uncertain result or access blocker. Remove the temporary body file after recovery/repair completes.

With the separate BB CLI:

- Query `bb pr list --workspace <workspace> --repository <repo> --state open --limit 50 --json` and match source/destination branches in the returned records. Its list command has no branch filter. If the limit is reached with no match, absence is unproven; use another supported lookup or report the coverage blocker.
- When absent, pass explicit `--workspace`, `--repository`, `--head`, `--base`, `--title`, `--body`, and `--no-default-reviewers` to `bb pr create`; omit `--reviewer`. Pass the body as one safely quoted argument. If installed help lacks `--no-default-reviewers`, use the interactive flow and select `Skip (no reviewers)`.
- Verify the returned PR with `bb pr view <id> --workspace <workspace> --repository <repo> --json`. On an uncertain create result, repeat the bounded lookup once; do not retry creation without proof of absence.

### GitHub CLI fallback path

Use this path only when the push URL points to a GitHub repository and `gh` is available.

1. Check once for an existing open PR:

   ```bash
   gh pr list --repo <owner/repo> --head <branch> --base <destination-branch> --state open --json url --limit 5
   ```

2. If no PR exists, create one without reviewers:

   ```bash
   gh pr create --repo <owner/repo> --title "<subject>" --body-file <description-file> --base <destination-branch> --head <branch>
   ```

   Resolve `<owner/repo>` from the verified push URL. Write the frozen body with real newlines to a temporary file and pass it with `--body-file`. Use the same explicit repository for any follow-up query.

3. Do not pass reviewer flags unless the user explicitly requested reviewers.
4. If `gh` is unavailable, report that blocker. If creation fails, query the same repository and branch once for an existing PR; report it if found, otherwise report the original creation failure. Do not treat a failed lookup as an empty result.

## Final response

Report:
- commit hash, or state that no new commit was needed
- subject / PR title
- branch
- push result
- PR result
