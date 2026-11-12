---
name: address-comments
description: Address actionable PR comments and publish coherent fixes
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Address actionable comments on the PR in `$ARGUMENTS`, or the current branch PR. With no arguments, discover the current branch PR and its open review comments yourself before asking the user to paste comments.

Invoking this skill authorizes the git writes this workflow needs: stage intended changes, create a coherent commit, and push the current branch to `origin` after validation, using a suitable branch or worktree for the verified PR source when needed. Do not open PRs, merge, resolve comment threads, or post PR replies unless explicitly requested. Stop instead of pushing if the current branch is `main`, `master`, or the resolved remote default branch, unless the user explicitly named that branch as the push target.

## Resolve the target

1. If `$ARGUMENTS` names a PR, comment, thread, file, or reviewer, narrow scope to it. Narrowing does not authorize unrelated cleanup.
2. Resolve the target PR's source repository, source branch, head SHA, and push destination, refreshing the source ref when needed. Use the matching checkout when suitable; otherwise reuse or create an isolated worktree from the verified PR source. Preserve unrelated local work and reconcile intended local fixes without discarding either history. Before pushing, verify that only coherent fixes extend the current PR head and that the push targets its source repository and branch. Ask only if the intended changes or destination remain ambiguous.
3. Fetch open or unresolved comments and threads. Skip stale, resolved, outdated, or superseded comments unless they still point to code in the effective diff.
4. The review surface is the PR/base diff plus unpushed commits and staged, unstaged, and relevant untracked changes; if the PR diff is empty but local changes exist, review those.
5. If there is no target PR, no accessible comment data, no actionable comments, or no coherent way to separate comment fixes from unrelated local changes, stop and report the blocker. Do not invent changes or push unrelated work.

## Classify from evidence

Classify every in-scope comment before editing:

- Correct: real defect, missing requirement, broken contract, repo convention violation, or meaningful test gap.
- Unclear: requested change is ambiguous after reading code, diff, tests, and requirements.
- Subjective: preference without a repo rule, product requirement, or concrete risk.
- Wrong: contradicted by code, tests, requirements, accepted branch intent, or a newer change.
- YAGNI: adds ceremony, options, abstraction, or polish not needed for this change.
- Already addressed: the effective diff or later commit already satisfies the comment.

Fix Correct comments. For Unclear comments, ask one concise clarifying question and leave them unmodified until answered. Ignore Subjective, Wrong, YAGNI, and Already addressed comments unless the user explicitly chooses them. No appeasement changes when the evidence says a comment is not Correct.

Group multiple Correct comments by root cause, fixing correctness, test, build, and API-contract blockers first, and re-check the rest after each group so duplicates are not handled twice.

## Fix constraints

Add or update a targeted test when the comment identifies a realistic regression, a test seam exists, and existing coverage does not already catch it. No unrelated refactors, style churn, or optional polish.

No suppressions, baselines, skipped/deleted/weakened tests, sleeps, broad catches, silent fallbacks, fake success exits, or code that dodges the reviewer concern. Change dependencies, build config, or test infra only when the comment evidence proves that is the minimal root-cause fix.

Each refetch or repush must follow a fix, a narrowed classification, or new comment or CI evidence. Stop with a concrete blocker instead of looping on inaccessible PR data, missing permissions, unavailable CI, or ambiguous reviewer intent.

## Verify and publish

1. Prove the addressed comments with the narrowest targeted checks, or manual proof when no automated seam exists. If no check applies or a broader check cannot run, name the reason.
2. If a new or changed automated check is part of the fix, prove when feasible that it fails for the original issue, then restore the fix and rerun green; reuse prior fail/pass proof only when it is same-diff and same-scope.
3. Stage only the comment fixes, isolating them from unrelated commits and local changes when necessary.
4. Commit with a grounded Conventional Commit subject and push explicitly to the proven source ref with `git push origin HEAD:<PR-source-branch>`, adding `-u` only when no upstream exists; do not rely on an implicit upstream refspec. If the fixes change what the PR does, regenerate its title and description with `branch-description` and replace them on the PR.
5. After pushing, refresh PR comments and branch-relevant checks. Continue until no actionable comments and no CI failures caused by the fixes remain, or a concrete blocker remains. Claim CI is green only when refreshed provider data is for the latest branch-relevant source SHA and every visible required check is terminal success; otherwise report the exact pending, red, missing, stale, or inaccessible check state as the blocker. Do not loop because the PR UI still shows an old comment the pushed diff already addresses.

## Final

- PR/comments: `<target PR or blocker>`
- Addressed: `<file:line/comment id -> change>`
- Ignored: `<file:line/comment id -> reason>`
- Needs clarification: `<file:line/comment id -> question>`
- Changed: `<files>`
- Proof: `<targeted checks/results or reused proof>`; `<broader CI/local result, reused proof, or blocker>`
- Publish/CI: `<pushed commit(s) and CI result | blocker>`
- Remaining: `<none or exact blocker>`
