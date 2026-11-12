---
name: git-publish
description: Commit, push, and open a no-reviewer PR for the current changes
---

Commit current staged, unstaged, and relevant untracked changes, push to `origin`, then create or report a no-reviewer pull request to the resolved PR destination branch.

Invoking this skill authorizes the git writes publishing needs: stage intended changes, commit, create a branch when needed, push, and create the PR. Ask for permission only when the changes are incoherent, risky, or ambiguous.

Avoid provider-generated titles, branch-name summaries, wrong-repo PRs, and duplicate PRs.

Non-default branches are normal publish sources. Never end with "PR not created" because the branch is non-default or already existed; after pushing, run the PR flow for it.

## Workflow

1. Before any write, inspect the repo, current branch, `HEAD`, `git remote get-url --push --all origin`, status, and the staged, unstaged, and relevant untracked changes.
2. `origin` must have exactly one verified push URL, since Git pushes to every configured push URL. Resolve missing or conflicting target information from task context and Git configuration before asking.
3. Resolve the PR destination branch local-first:
   - Use `refs/remotes/origin/HEAD` only when it resolves to an existing local `refs/remotes/origin/<branch>`.
   - Otherwise use local `origin/main` or `origin/master` only when exactly one exists.
   - Missing refs, a dangling or stale `origin/HEAD`, or both `main` and `master` without a valid `origin/HEAD` are ambiguous. Only then run live remote discovery; if it still finds no single destination, stop before PR creation and report the exact blocker instead of guessing.
4. Determine the PR provider from the sole push URL. If fetch and push URLs point at incompatible providers or repositories so the review target is unsafe to identify, stop before PR creation and report it.
5. The effective changes are the branch diff (commits on `HEAD` not in the destination, including local commits on the default branch before a publish branch exists) plus the working-tree diff. If both are empty, stop.
6. Publish the coherent changes the user requested. When unrelated work is present, isolate the intended changes on a suitable branch or worktree and preserve the rest. Ask only when the intended scope cannot be determined.
7. Write the PR title and body with `branch-description` from the combined effective changes, and keep them only after its `check_pr_description.py` check passes. The commit subject is that title without repository `[tag]` prefixes. Freeze this title/body pair for every provider path below.
8. If HEAD is detached or on the destination/default branch, create a focused source branch at the current commit before committing, keeping existing commits and working-tree changes; pick an unused name without asking. Reuse a suitable existing branch or worktree when continuing its work. Never push directly to the destination branch.
9. Stage intended working-tree changes, excluding unrelated files, and commit with the chosen subject. If there is nothing to commit but a branch diff exists, skip the commit.
10. Push the source branch to `origin`, setting upstream if needed, then always create or report a no-reviewer PR to the resolved destination using the PR flow below.
11. After the PR exists, run `clear-merge-blockers` on it. Its one loop fixes red CI and addresses unresolved review comments, including ones that arrive while CI runs, until CI is green on the latest SHA and nothing actionable remains, or an exact blocker remains. Do not rely on the host to watch CI or comments; it may not track the provider.

Do not run tests or builds before publishing unless explicitly asked.

## PR flow

Choose tools by the provider of the sole push URL: Bitbucket flows only for Bitbucket-compatible repositories, `gh` only for GitHub. If the provider is unsupported, repository identity is unsafe for review targeting, or tooling is unavailable, stop after the push and report the exact PR blocker instead of trying a wrong-provider command.

Never use Bitbucket MCP to create, list, inspect, or update pull requests during publish. Do not add reviewers unless the user explicitly requested them.
Do not create a new PR with a missing body or one that fails the `branch-description` check.

### Provider-compatible managed PR path, when available

Use a harness-provided PR facility for the detected provider only when it can inspect repository identity and existing branch PRs, carry fresh title/body metadata, and create or idempotently ensure a PR with explicit source and destination branches. Otherwise use the CLI fallback below.

1. After pushing, inspect PR context for the root from `git rev-parse --show-toplevel`, passing the workspace root when supported. The reported git root, provider, source branch, and destination must match the repository being published.
2. Supply the frozen title and body plus the exact diff fingerprint returned by the inspection step.
3. Create or idempotently ensure the PR with explicit source and destination branches and no reviewers unless requested.
4. If the tool reports an existing PR for the branch, report it and stop. Do not create a duplicate.
5. If it reports stale or wrong-repo metadata, regenerate metadata for the shell-reported root and retry once; then stop with the exact blocker.

### Bitbucket CLI fallback path: no-reviewer PR

Pass the verified push workspace and repository explicitly on every call. Prefer `twg`; if it is unavailable or its credentials fail, use an authenticated `bb` client after inspecting its installed help. Do not switch providers or rely on remote auto-detection.

With TWG:

```bash
twg bb prs query --workspace <workspace> --repo <repo> --source <branch> --dest <destination-branch> --state OPEN -n 5 -o json
```

A successful query with zero matching rows proves absence; empty stdout, errors, or inaccessible output do not. Report an existing matching PR instead of creating another.

If absent, write the frozen body with real newlines to a private temporary file and create:

```bash
twg bb prs create --workspace <workspace> --repo <repo> --title "<subject>" --source <branch> --dest <destination-branch> --description-file <description-file> -o json
```

Omit reviewer flags unless requested. Check the returned description and reviewers. If creation succeeded but the description is blank, update that PR once with `twg bb prs update --workspace <workspace> --repo <repo> --pull-request <id> --description-file <description-file>`, then verify with `twg bb prs get <id> --workspace <workspace> --repo <repo> --full -o json`. Never issue another create to repair metadata. Report an unexpected reviewer or failed repair rather than claiming no-reviewer success.

If creation fails without returning a PR, query the same source/destination once. Report a recovered PR after checking its description. Retry create once only if the successful lookup proves absence; otherwise report the uncertain result or access blocker. Remove the temporary body file afterward.

With the separate BB CLI:

- Query `bb pr list --workspace <workspace> --repository <repo> --state open --limit 50 --json` and match source/destination in the records; it has no branch filter. If the limit is reached with no match, absence is unproven; use another supported lookup or report the coverage blocker.
- When absent, pass explicit `--workspace`, `--repository`, `--head`, `--base`, `--title`, `--body`, and `--no-default-reviewers` to `bb pr create`; omit `--reviewer`. Pass the body as one safely quoted argument. If installed help lacks `--no-default-reviewers`, use the interactive flow and select `Skip (no reviewers)`.
- Verify with `bb pr view <id> --workspace <workspace> --repository <repo> --json`. On an uncertain create result, repeat the bounded lookup once; never retry creation without proof of absence.

### GitHub CLI fallback path

Use only when the push URL is a GitHub repository and `gh` is available. Resolve `<owner/repo>` from the verified push URL and pass it on every call.

1. Check once for an existing open PR:

   ```bash
   gh pr list --repo <owner/repo> --head <branch> --base <destination-branch> --state open --json url --limit 5
   ```

2. If none exists, write the frozen body with real newlines to a temporary file and create one without reviewer flags:

   ```bash
   gh pr create --repo <owner/repo> --title "<subject>" --body-file <description-file> --base <destination-branch> --head <branch>
   ```

3. If `gh` is unavailable, report that blocker. If creation fails, query once for an existing PR; report it if found, otherwise the original failure. A failed lookup is not an empty result.

## Final response

Report:
- commit hash, or state that no new commit was needed
- subject / PR title
- branch
- push result
- PR result
- CI result: green, or the exact remaining blocker
- Comments: addressed, none actionable, or the exact remaining blocker
