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

Use this path only when canonical PR tools are unavailable or the repository/tooling is not compatible with the current harness's PR tools.

1. Check once for an existing open PR:

   ```bash
   twg bb prs query --source <branch> --dest <destination-branch> -n 5
   ```

   Treat empty output, `[]`, `No pull requests found.`, or output containing `Found 0 pull requests` as no existing PR.

2. If no PR exists, write the frozen body with real newlines to a private temporary Markdown file, then create one with no reviewers:

   ```bash
   twg bb prs create --title "<subject>" --source <branch> --dest <destination-branch> --description-file <description-file> -o json
   ```

   The description file must be non-empty. Require the returned JSON `description` to be non-empty before reporting success. If the create response is otherwise successful but its description is blank or missing, update that newly created PR once with `twg bb prs update --pull-request <id> --description-file <description-file>`, then require `twg bb prs get <id> --full -o json` to return a non-empty `description`; never issue a second create to repair metadata. If description repair or verification fails, stop with the exact PR metadata blocker; do not enter the create-failure retry path or report PR success. Keep the temporary file through any permitted retry and remove it after the entire create/failure flow.

3. Do not pass reviewer flags. Create PRs with no reviewers unless the user explicitly requested reviewers.
4. If `twg` is unavailable, use `bb pr create` only through its interactive flow, supply the same frozen title/body, and select `Skip (no reviewers)`. Do not use fully specified non-interactive `bb pr create`, because it may apply default reviewers.
5. If the create command fails without returning a created PR:
   - check once for an existing branch PR with whichever CLI is available
   - if found, report it and stop only after confirming its description is non-empty; after a failed `twg` create, repair a blank or missing description once with the same description file and verify it through `twg bb prs get` before reporting success
   - if description confirmation or repair fails, stop with the exact PR metadata blocker rather than retrying create
   - if no PR is found and the failed create path used `twg`, retry once with the no-reviewer `twg` Bitbucket create path
   - if no PR is found, `twg` was unavailable, and the interactive `bb pr create` fallback failed, report the exact `bb` blocker
   - if it still fails, stop and report the exact blocker

### GitHub CLI fallback path

Use this path only when the push URL points to a GitHub repository and `gh` is available.

1. Check once for an existing open PR:

   ```bash
   gh pr list --head <branch> --base <destination-branch> --state open --json url --limit 5
   ```

2. If no PR exists, create one without reviewers:

   ```bash
   gh pr create --title "<subject>" --body "<body>" --base <destination-branch> --head <branch>
   ```

   Pass the same frozen, grounded body rather than provider defaults.

3. Do not pass reviewer flags unless the user explicitly requested reviewers.
4. If `gh` is unavailable or PR creation fails, check once for an existing branch PR with `gh pr list`; if none is found, stop and report the exact blocker.

## Final response

Report:
- commit hash, or state that no new commit was needed
- subject / PR title
- branch
- push result
- PR result
