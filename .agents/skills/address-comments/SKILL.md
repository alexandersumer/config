---
name: address-comments
description: Address actionable PR comments and publish coherent fixes
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Address actionable comments on `pr_target`, or the current branch PR. With no arguments, discover the current branch PR and its open review comments yourself before asking the user to paste comments.

Invoking this skill is explicit authorization to perform the git writes required for this PR-comment workflow: stage intended changes, create a coherent commit, and push the current branch to `origin` after validation. Do not create branches, open PRs, merge, resolve comment threads, or post PR replies unless explicitly requested. Stop instead of pushing if the current branch is `main`, `master`, or the resolved remote default branch, unless the user explicitly named that branch as the push target.

## Resolve the target

1. If `pr_target`, `$ARGUMENTS`, or `focus` names a PR, comment, thread, file, or reviewer, use that to narrow scope. `focus` narrows the review surface; it does not authorize unrelated cleanup.
2. With no supplied target, inspect repository and SCM context: current branch, upstream, remote default branch, working tree diff, unpushed commits, and the current branch PR when available.
3. Fetch open or unresolved review comments and comment threads for the target PR. Prefer current, unresolved comments over stale, resolved, outdated, or superseded comments; keep stale comments only when they still point to code present in the effective diff.
4. Read the effective review surface before editing: PR/base diff, unpushed commit diff, `git diff --cached`, `git diff`, relevant untracked files rendered or summarized as new-file diffs, and nearby file context. If the PR diff is empty but staged, unstaged, or untracked changes exist, include those changes instead of treating the review surface as empty.
5. If there is no target PR, no accessible comment data, no actionable comments, or no coherent way to separate comment fixes from unrelated local changes, stop and report the blocker. Do not invent changes or push unrelated work.

## Classify from evidence

Classify every in-scope comment before editing:

- Correct: real defect, missing requirement, broken contract, repo convention violation, or meaningful test gap.
- Unclear: requested change is ambiguous after reading code, diff, tests, and requirements.
- Subjective: preference without a repo rule, product requirement, or concrete risk.
- Wrong: contradicted by code, tests, requirements, accepted branch intent, or a newer change.
- YAGNI: adds ceremony, options, abstraction, or polish not needed for this change.
- Already addressed: the effective diff or later commit already satisfies the comment.

Fix Correct comments. For Unclear comments, ask one concise clarifying question and leave them unmodified until answered. Ignore Subjective, Wrong, YAGNI, and Already addressed comments unless the user explicitly chooses them. Do not make appeasement changes just to satisfy a reviewer if the evidence says the comment is not Correct.

For multiple Correct comments, group them by root cause or file area. Address one coherent group at a time, starting with comments that block correctness, tests, build, or API contracts. Re-check remaining comments after each group so duplicate or superseded comments are not handled twice.

## Fix constraints

Inspect the real seam before editing: commented lines, surrounding code, owning interface or state/model, representative callers, adjacent tests, and 2-3 sibling patterns when available.

Make the smallest production, test, fixture, doc, or config change that truly addresses each Correct comment. Add or update a targeted test when the comment identifies a realistic regression and a test seam exists. Do not broaden scope to unrelated refactors, style churn, optional polish, or reviewer preferences.

No suppressions, baselines, skipped/deleted/weakened tests, sleeps, broad catches, silent fallbacks, fake success exits, or code that dodges the reviewer concern. Dependency, build config, or test-infra changes are allowed only when the comment evidence proves they are the minimal root-cause fix.

Do not blindly refetch or repush. Each loop must follow a fix, narrow classification, or expose new comment or CI evidence. Stop with a concrete blocker instead of looping on inaccessible PR data, missing permissions, unavailable CI, or ambiguous reviewer intent.

## Verify and publish

1. Run or reuse fresh proof for the narrowest targeted checks that prove the addressed comments: unit test, typecheck, lint, schema validation, build step, or manual proof when no automated seam exists.
2. Run broader checks only when the proof policy justifies them. If no check applies or a broader check cannot run, name the exact reason or narrower proof used.
3. If a new or changed automated check is part of the fix, prove when feasible that it fails for the original issue, then restore the fix and rerun green; reuse prior fail/pass proof only when it is same-diff and same-scope.
4. Stage only intended comment-fix changes. If pre-existing unpushed commits or local changes are unrelated to the comment fixes, stop and ask how to split or scope the publish.
5. Commit with a grounded Conventional Commit subject and push the current branch to `origin`, setting upstream if needed.
6. After pushing, refresh PR comments and branch-relevant checks. Continue for newly actionable comments or CI failures caused by the comment fixes until there are no remaining actionable comments, CI is green, or a concrete blocker remains. Do not loop solely because the PR UI still shows an old comment that the pushed diff already addresses.

## Final

- PR/comments: `<target PR or blocker>`
- Addressed: `<file:line/comment id -> change>`
- Ignored: `<file:line/comment id -> reason>`
- Needs clarification: `<file:line/comment id -> question>`
- Changed: `<files>`
- Proof: `<targeted checks/results or reused proof>`; `<broader CI/local result, reused proof, or blocker>`
- Publish/CI: `<pushed commit(s) and CI result | blocker>`
- Remaining: `<none or exact blocker>`
