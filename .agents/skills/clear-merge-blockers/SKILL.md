---
name: clear-merge-blockers
description: Clear failing checks and actionable review comments until the branch is merge-ready or a concrete blocker remains.
---

Clear merge blockers for `target`, `$ARGUMENTS`, or the current branch's review. A blocker is either a branch-relevant failing check or an actionable review comment that identifies a real defect, missing requirement, broken contract, convention violation, or meaningful test gap.

Invoking this skill is explicit authorization to perform the git writes required for this finish-the-review workflow: stage intended fixes, create coherent commits, push the current branch to `origin`, refresh review comments and branch checks, and continue until no blockers remain or an exact blocker is reached. Do not create branches, open reviews, merge, force-push, resolve comment threads, or post review replies unless explicitly requested. Stop instead of pushing if the current branch is `main`, `master`, or the resolved remote default branch, unless the user explicitly named that branch as the push target.

## Resolve the target

1. If `target`, `$ARGUMENTS`, or `focus` names a review, check, job, run, comment, thread, file, reviewer, or pasted failure output, use it to narrow scope. `focus` narrows the surface; it does not authorize unrelated cleanup.
2. With no supplied target, inspect repository and review context: current branch, upstream, remote default branch, working tree diff, unpushed commits, current branch review, open or unresolved comments, and branch-relevant check status.
3. Read the effective review surface before editing: base comparison diff, unpushed commit diff, `git diff --cached`, `git diff`, relevant untracked files rendered or summarized as new-file diffs, and nearby file context. If the review diff is empty but staged, unstaged, or untracked changes exist, include those changes instead of treating the surface as empty.
4. Prefer the latest branch-relevant check run and current unresolved comments. Ignore stale, resolved, outdated, superseded, or other-branch signals unless they still describe code present in the effective diff.
5. If there is no target review, no accessible comment or check data, no actionable blocker, or no coherent way to separate blocker fixes from unrelated local changes, stop and report the blocker. Do not invent changes.

## Classify before editing

Classify every in-scope check signal:

- Real failure: deterministic branch-relevant test, build, lint, type, schema, deployment, or validation failure.
- Flaky or infrastructure: timeout, worker loss, network issue, rate limit, dependency outage, or known nondeterministic failure not caused by this branch.
- Stale or superseded: older run, outdated commit, resolved job, or failure already invalidated by later evidence.
- Unrelated or pre-existing: failure not caused by this branch and not required to prove this branch.
- Blocked: logs, permissions, environment, or product intent are insufficient to identify a safe fix.

Classify every in-scope review comment:

- Correct: real defect, missing requirement, broken contract, repo convention violation, or meaningful test gap.
- Unclear: requested change is ambiguous after reading code, diff, tests, and requirements.
- Subjective: preference without a repo rule, product requirement, or concrete risk.
- Wrong: contradicted by code, tests, requirements, accepted branch intent, or newer changes.
- YAGNI: adds ceremony, options, abstraction, or polish not needed for this change.
- Already addressed: the effective diff or later commit already satisfies the comment.

Fix Real failures and Correct comments. For Unclear items, ask one concise clarifying question and leave them unmodified until answered. Ignore Flaky, Infrastructure, Stale, Superseded, Subjective, Wrong, YAGNI, Already addressed, and Unrelated items unless the user explicitly chooses them. Do not appease a reviewer or hide a failing gate when the evidence says the item is not a real blocker.

For multiple blockers, group by likely root cause or file area. Address one coherent group at a time, starting with correctness, API contracts, data safety, build/test failures, and comments that block reliable verification. Re-check remaining blockers after each group so duplicates and superseded items are not handled twice.

## Fix constraints

Before editing each group, inspect the real seam: failing command or job, failing code or config path, commented lines, owning interface or state/model, representative callers or consumers, adjacent tests, and 2-3 sibling patterns when available.

Make the smallest production, test, fixture, documentation, configuration, or owned infrastructure change that truly clears the blocker. Add or update a targeted test when the blocker identifies a realistic regression and a test seam exists.

No suppressions, baselines, skipped/deleted/weakened tests, sleeps, broad catches, silent fallbacks, fake success exits, retry wrappers that hide deterministic defects, code that dodges a checker, or reviewer-appeasement changes. Dependency, build config, or test-infra changes are allowed only when the captured evidence proves they are the minimal root-cause fix.

Do not broaden scope to unrelated refactors, style churn, optional polish, opportunistic migrations, or failures outside this branch. Do not blindly refetch, rerun, or repush. Each loop must follow a fix, narrow the hypothesis, or expose new blocker evidence. Stop with a concrete blocker instead of looping on inaccessible logs, missing permissions, unavailable checks, flaky infrastructure, or ambiguous intent.

## Verify, publish, and loop

1. Reproduce locally with the narrowest useful command when feasible. If reproduction is review-only, environment-specific, permission-limited, or too expensive, proceed from captured review/check evidence and say why local reproduction was skipped.
2. Run the targeted proof for the fixed group: unit test, job-equivalent command, typecheck, lint, schema validation, build step, or manual proof when no automated seam exists.
3. Compare the original failing or commented signal with the new passing signal, changed diagnostic, or code evidence that directly addresses the concern.
4. Run broader relevant checks when touched surface justifies it. If no check applies or a broader check cannot run, name the exact reason.
5. If adding or changing an automated check, prove when feasible that it fails for the original bug, then restore the fix and rerun green.
6. Stage only intended blocker-fix changes. If pre-existing unpushed commits or local changes are unrelated to the blocker fixes, stop and ask how to split or scope the publish.
7. Commit with a grounded Conventional Commit subject and push the current branch to `origin`, setting upstream if needed.
8. After pushing, refresh open or unresolved review comments and branch-relevant checks. Continue for newly actionable blockers caused by this branch until checks are green, comments are addressed, or a concrete blocker remains. Do not loop solely because the review UI still shows an old comment that the pushed diff already addresses.

## Done-done gate

Before claiming completion, verify all are true or report the exact blocker:

- The target review and branch were identified.
- All Real branch-relevant check failures are passing, superseded by green newer runs, or classified with evidence as flaky, infrastructure, stale, unrelated, or blocked.
- All Correct actionable comments are addressed, and every ignored comment has an evidence-backed classification.
- Any Unclear item has a concise question or explicit blocker.
- The latest pushed commit set is coherent and contains only intended blocker fixes.
- Targeted proof was run for each fixed group, and broader proof or remote check status was refreshed when relevant.
- Final status and diff were inspected for accidental files, secrets, debug output, generated noise, and unrelated edits.

## Final

- Target: `<review/branch or blocker>`
- Cleared checks: `<check/job -> evidence>`
- Addressed comments: `<file:line/comment id -> change>`
- Ignored or classified: `<item -> reason>`
- Needs clarification: `<item -> question>`
- Changed: `<files>`
- Proof: `<targeted checks/results>`; `<broader check/review status or blocker>`
- Publish: `<commit(s), push result, refreshed status | blocker>`
- Remaining: `<none or exact blocker>`
