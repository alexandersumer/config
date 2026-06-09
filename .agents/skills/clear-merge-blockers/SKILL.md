---
name: clear-merge-blockers
description: Clear PR merge blockers including checks, actionable review comments, conflicts, branch update requirements, and required gates until the branch is merge-ready or a concrete blocker remains.
---

Clear merge blockers for `target`, `$ARGUMENTS`, or the current branch's review. Merge blockers include branch-relevant required checks, actionable review comments or tasks, merge conflicts, target-branch update requirements, merge queue or synthetic-merge failures, required approvals or changes-requested state, draft state, Jira/compliance/custom merge checks, and permission/tooling blockers.

A request to study, review, explain, or plan this skill or a blocker surface does not authorize edits, commits, merges, pushes, PR state changes, or check reruns. A request to clear, fix, resolve, apply, publish, or iterate merge blockers is explicit authorization to perform the git writes required for this finish-the-review workflow: stage intended fixes, create coherent commits, fetch the target branch, merge the remote target branch into the current source branch when that is the safe repo-compatible update strategy, push the current branch to its configured upstream or `origin`, refresh review state and branch checks, and continue until no local-fixable blockers remain or an exact blocker is reached.

Do not create branches, open reviews, merge PRs, force-push, reset, discard, stash, rebase a published branch, resolve comment threads, dismiss reviews, approve, or post review replies unless explicitly requested. Stop before any write if the current branch is `main`, `master`, or the resolved remote default branch, unless the user explicitly named that branch as the write target.

## Resolve the target and mergeability surface

1. If `target`, `$ARGUMENTS`, or `focus` names a review, check, job, run, comment, thread, file, reviewer, conflict, branch-update warning, merge queue result, pasted failure output, or required gate, use it to narrow scope. `focus` narrows the surface; it does not authorize unrelated cleanup.
2. With no supplied target, inspect repository and review context: current branch, upstream, remote default branch, PR/review source and target branches, latest source/head SHA, working tree diff, unpushed commits, current branch review, open or unresolved comments/tasks, review decision state, branch-relevant check status, required-gate status, mergeability/conflict status, out-of-date/update-required status, draft state, and merge queue or synthetic-merge status when available.
3. Read the effective review surface before editing: base comparison diff, synthetic merge diagnostics when relevant, unpushed commit diff, `git diff --cached`, `git diff`, relevant untracked files rendered or summarized as new-file diffs, and nearby file context. If the review diff is empty but staged, unstaged, or untracked changes exist, include those changes instead of treating the surface as empty.
4. Prefer the latest signal for the latest source/head SHA and, when the provider uses one, the latest merge queue or synthetic-merge SHA. Ignore stale, resolved, outdated, superseded, or other-branch signals unless they still describe code or required state present in the effective diff.
5. If there is no target review, no accessible comment/check/mergeability data, no actionable blocker, or no coherent way to separate blocker fixes from unrelated local changes, stop and report the blocker. Do not invent changes.

## Classify before editing

Classify every in-scope check or required-gate signal:

- Real failure: deterministic branch-relevant required or important test, build, lint, type, schema, deployment, custom gate, merge queue, synthetic-merge, or validation failure.
- Missing, skipped, canceled, or pending required gate: required status is absent, skipped, canceled, still running, queued, or waiting in a way that blocks merge on the latest relevant SHA.
- Flaky or infrastructure: timeout, worker loss, network issue, rate limit, dependency outage, or known nondeterministic failure not caused by this branch.
- Stale or superseded: older run, outdated commit, resolved job, replaced merge queue attempt, or failure already invalidated by later evidence.
- Unrelated or pre-existing: failure not caused by this branch and not required to prove or merge this branch.
- Blocked: logs, permissions, environment, provider status, or product intent are insufficient to identify a safe fix.

Classify every in-scope review comment or task:

- Correct: real defect, missing requirement, broken contract, repo convention violation, or meaningful test gap.
- Unclear: requested change is ambiguous after reading code, diff, tests, and requirements.
- Subjective: preference without a repo rule, product requirement, or concrete risk.
- Wrong: contradicted by code, tests, requirements, accepted branch intent, or newer changes.
- YAGNI: adds ceremony, options, abstraction, or polish not needed for this change.
- Already addressed: the effective diff or later commit already satisfies the comment.

Classify every in-scope mergeability or review-state signal:

- Conflict: provider or local merge reports content conflicts between the source branch and target branch.
- Update required: provider requires the source branch to include the latest target branch before merge, with or without conflicts.
- Human review required: missing required approval, unresolved changes-requested state, unresolved required task, draft PR state, or reviewer action that cannot be cleared by a code change alone.
- Policy or compliance required: Jira issue, deployment, security, compliance, ownership, permission, or custom merge check must be satisfied outside normal code/test fixes.
- Tooling or permission blocked: required PR, check, queue, comment, or mergeability data cannot be fetched or updated with available credentials/tools.
- Stale or superseded: state belongs to an older source SHA, older target branch, older review decision, or replaced merge queue attempt.

Fix Real failures, Correct comments, local-fixable conflicts, and update-required states that can be cleared safely. For Unclear items, ask one concise clarifying question and leave them unmodified until answered. Do not appease a reviewer or hide a failing gate when the evidence says the item is not a real blocker. If a flaky, infrastructure, unrelated, subjective, wrong, YAGNI, already-addressed, or human-review item still blocks the provider merge button, report it separately as remaining merge state rather than claiming merge-ready.

For multiple blockers, group by likely root cause or file area. Address the currently known local-fixable blockers in the smallest coherent batch that can be validated together, starting with conflicts, update requirements, correctness, API contracts, data safety, build/test failures, and comments that block reliable verification. Re-check remaining blockers after each batch so duplicates and superseded items are not handled twice.

## Safe conflict and target-branch update handling

Use this section when the branch is conflicted, behind the target branch, blocked by an update-required policy, or failing only on a merge queue or synthetic merge result.

1. Confirm the current branch is the review source branch and identify the exact remote target branch from the PR/review when available; otherwise use the resolved remote default branch. Do not merge local `main`, local `master`, or any other local target branch.
2. Inspect local status and diffs before the update. Do not start a target-branch merge with unrelated staged, unstaged, or untracked changes. If existing local changes are intended blocker fixes, commit or otherwise finish that coherent batch first; if they are unrelated or inseparable, stop and ask how to split or scope the work.
3. Fetch the exact remote target branch needed for the update. Determine the repo or PR-preferred update strategy from repository docs, branch policy, PR settings, or sibling practice when available.
4. If policy requires rebase, linear history, force-push, destructive reset, or a different history-rewriting update, stop unless the user explicitly authorized that operation. Otherwise, merge the remote target branch into the current source branch without rewriting branch history.
5. If conflicts occur, resolve them inline while preserving current branch intent and incorporating incoming target-branch changes. Do not choose one side wholesale just to remove markers. Read conflict context and enough history from both sides; accept incoming removals of feature flags, dead code, deprecated APIs, or temporary constructs this branch did not introduce, then adapt branch code to the removal.
6. After editing conflicts, search for `<<<<<<<`, `=======`, and `>>>>>>>`. Run the most relevant available checks for the merge result. If checks expose merge-caused failures, fix them before completing/publishing or report the exact blocker.
7. Stage only resolved/intended files and complete the merge or update commit only after conflict markers are gone and known merge-caused failures are handled.

## Fix constraints

Before editing each batch, inspect the real seam: failing command or job, failing code or config path, commented lines, conflict files, owning interface or state/model, representative callers or consumers, adjacent tests, and 2-3 sibling patterns when available.

Make the smallest production, test, fixture, documentation, configuration, or owned infrastructure change that truly clears the blocker. Add or update a targeted test when the blocker identifies a realistic regression and a test seam exists.

No suppressions, baselines, skipped/deleted/weakened tests, sleeps, broad catches, silent fallbacks, fake success exits, retry wrappers that hide deterministic defects, code that dodges a checker, or reviewer-appeasement changes. Dependency, build config, or test-infra changes are allowed only when the captured evidence proves they are the minimal root-cause fix.

Do not broaden scope to unrelated refactors, style churn, optional polish, opportunistic migrations, or failures outside this branch. Do not blindly refetch, rerun, repush, or restart merge queues. Each loop must follow a fix, a target-branch update, a narrowed hypothesis, or new blocker evidence. Stop with a concrete blocker instead of looping on inaccessible logs, missing permissions, unavailable checks, flaky infrastructure, unresolved human review state, or ambiguous intent.

## Verify, publish, and loop

1. Reproduce locally with the narrowest useful command when feasible. If reproduction is review-only, provider-specific, environment-specific, permission-limited, or too expensive, proceed from captured review/check/mergeability evidence and say why local reproduction was skipped.
2. Run the targeted proof for the fixed batch: unit test, job-equivalent command, typecheck, lint, schema validation, build step, conflict-marker search, local merge proof, or manual proof when no automated seam exists.
3. Compare the original failing, conflicted, update-required, or commented signal with the new passing signal, changed diagnostic, resolved merge result, or code evidence that directly addresses the concern.
4. Run broader relevant checks when touched surface justifies it. If no check applies or a broader check cannot run, name the exact reason.
5. If adding or changing an automated check, prove when feasible that it fails for the original bug, then restore the fix and rerun green.
6. Stage only intended blocker-fix or target-update changes. If pre-existing unpushed commits or local changes are unrelated to the blocker fixes, stop and ask how to split or scope the publish.
7. Commit with a grounded Conventional Commit subject when a normal fix commit is needed; allow the default merge commit message for a target-branch merge commit when it accurately records the update. Push the current source branch to its configured upstream, or to `origin` with upstream set if none exists and `origin` is the appropriate branch remote.
8. After pushing, refresh PR/review state, open or unresolved review comments/tasks, required gates, branch-relevant checks, mergeability, and merge queue/synthetic-merge status for the latest pushed source SHA. Continue for newly actionable local-fixable blockers caused by this branch until required checks are green, conflicts are gone, comments are addressed or classified, and no local-fixable merge blockers remain.
9. For pending, queued, canceled, missing, skipped, flaky, or infrastructure required gates, use a bounded wait or single safe rerun only when provider convention and permissions make that appropriate. Otherwise report the exact remaining gate; do not claim merge-ready while a required provider gate is unsatisfied.

Do not loop solely because the review UI still shows an old comment that the pushed diff already addresses. Do not claim fixed, complete, passing, unblocked, or merge-ready unless the latest checked source SHA and, when relevant, merge queue or synthetic-merge SHA support that claim.

## Done-done gate

Before claiming completion, verify all are true or report the exact blocker:

- The target review, source branch, target branch, latest source SHA, and branch update strategy were identified.
- No merge conflicts or target-branch update requirements remain, or any remaining conflict/update requirement is blocked for a stated reason.
- All Real branch-relevant required check failures are passing on the latest relevant SHA, superseded by green newer runs, or classified with evidence as stale, unrelated, or blocked.
- Flaky or infrastructure failures are distinguished from branch-caused failures, and any required red gate they leave behind is reported as still merge-blocking.
- All Correct actionable comments/tasks are addressed, and every ignored comment/task has an evidence-backed classification.
- Any Unclear item has a concise question or explicit blocker.
- Required approvals, changes-requested state, draft state, Jira/compliance/custom gates, and permissions are satisfied or reported as human/policy/tooling blockers.
- The latest pushed commit set is coherent and contains only intended blocker fixes and target-branch update commits.
- Targeted proof was run for each fixed batch, and broader proof or remote status was refreshed when relevant.
- Final status and diff were inspected for accidental files, secrets, debug output, generated noise, and unrelated edits.

## Final

- Target: `<review/branch or blocker>`
- Latest checked SHA: `<source SHA and merge/synthetic SHA when relevant>`
- Mergeability: `<clean | conflicts resolved | update handled | still blocked>`
- Conflict/update handling: `<none | merged target branch | files resolved | blocker>`
- Cleared checks: `<check/job/gate -> evidence>`
- Addressed comments: `<file:line/comment id -> change>`
- Ignored or classified: `<item -> reason, noting whether it still blocks provider merge>`
- Required gates: `<satisfied | pending/missing/red/human/policy/tooling blocker>`
- Review state: `<approvals/changes-requested/tasks/draft state or blocker>`
- Needs clarification: `<item -> question>`
- Changed: `<files>`
- Proof: `<targeted checks/results>`; `<broader check/review/mergeability status or blocker>`
- Publish: `<commit(s), push result, refreshed status | blocker>`
- Human action required: `<none or exact reviewer/policy/permission action>`
- Remaining: `<none or exact blocker>`
