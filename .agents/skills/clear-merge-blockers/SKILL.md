---
name: clear-merge-blockers
description: "Clear PR merge blockers by driving one concrete blocker at a time: failed CI, actionable review comments/tasks, conflicts, branch update requirements, or exact human/policy/tooling blockers."
---

Clear merge blockers for `target`, `$ARGUMENTS`, or the current branch's review. Optimize for the next local-fixable blocker, not for a full PR audit.

A request to study, review, explain, or plan this skill or a blocker surface does not authorize edits, commits, merges, pushes, PR state changes, check reruns, waits, or queue actions. A request to clear, fix, resolve, apply, publish, or iterate merge blockers is explicit authorization to perform only the actions required by this finish-the-review workflow: stage intended blocker fixes, create coherent commits, fetch the exact remote target branch, merge the remote target branch into the current source branch when that is the safe repo-compatible update strategy, push the current branch to its configured upstream or `origin`, perform one safe required-check rerun or one bounded wait for the active blocker when provider convention and permissions make that appropriate, refresh the blocker snapshot, and continue until no local-fixable blockers remain or an exact blocker is reached.

Do not create branches, open reviews, merge PRs, force-push, reset, discard, stash, rebase a published branch, resolve comment threads, dismiss reviews, approve, post review replies, or merge/restart a merge queue unless explicitly requested. Stop before any write if the current branch is `main`, `master`, or the resolved remote default branch, unless the user explicitly named that branch as the write target.

## Operating model: one active blocker

Every iteration must have exactly one active blocker before deep investigation or editing:

- Type: `conflict`, `update-required`, `failed-check`, `comment-task`, `missing-or-pending-gate`, `human-policy`, or `tooling-blocked`.
- Id: PR/review id plus check/job/pipeline UUID, comment/task id, conflict path, gate name, or mergeability state.
- Latest relevant SHA: source/head SHA and, when applicable, merge queue or synthetic-merge SHA.
- Original evidence: the exact provider status, failing diagnostic, comment text, conflict marker/file, or policy message.
- Next local action: fix, merge target, rerun/wait once, ask a question, or report a non-local blocker.

No blocker id, no work. No original evidence, no fix. No diff, completed target update, provider-state change, successful rerun, or new blocker evidence, no claim of progress. Do not perform another full PR refresh unless a code/status change was made, a targeted check/rerun completed, or the previous evidence became stale or insufficient.

## Fast blocker snapshot

1. If `target`, `$ARGUMENTS`, or `focus` names a PR/review, check, job, run, comment, task, file, conflict, branch-update warning, merge queue result, pasted failure output, or required gate, make that the active blocker and inspect only enough surrounding review state to act safely.
2. With no supplied target, collect one bounded blocker snapshot: repository, current branch, upstream, remote default branch, working tree status, unpushed commits, PR/review id, source branch, target branch, latest source/head SHA, mergeability/conflict or update-required state, failing or unsatisfied required checks, unresolved required comments/tasks, draft/approval/changes-requested state, and Jira/compliance/custom gate messages when available.
3. Do not read the full PR diff, all untracked files, all comments, or all check logs during the snapshot. Read code, diffs, and logs only for the selected active blocker.
4. Prefer the latest signal for the latest source/head SHA and, when the provider uses one, the latest merge queue or synthetic-merge SHA. Ignore stale, resolved, outdated, superseded, or other-branch signals unless they still describe code or required state present in the selected blocker.
5. If there is no target review, no accessible blocker data, no actionable local-fixable blocker, or no coherent way to separate blocker fixes from unrelated local changes, stop and report the exact blocker. Do not invent changes.

Choose the first local-fixable blocker in this order unless the user named a narrower focus:

1. Conflicts or branch-update-required states.
2. Failed required or merge-blocking checks, including merge queue or synthetic-merge failures.
3. Required unresolved review comments or tasks.
4. Missing, skipped, canceled, queued, or pending required gates that can be safely rerun or waited on once.
5. Human review, draft, Jira, compliance, custom policy, permission, or tooling blockers.

Once a blocker class is absent for the current source SHA, do not revisit that class until after a push, check rerun, target update, or explicit user focus. For example, if unresolved comments/tasks are zero for SHA `abc`, do not keep checking comments while fixing a CI blocker on SHA `abc`.

## Blocker playbooks

### Conflict or update required

Use this path when the branch is conflicted, behind the target branch, blocked by an update-required policy, or failing only on a merge queue or synthetic merge because of target-branch integration.

1. Confirm the current branch is the review source branch and identify the exact remote target branch from the PR/review when available; otherwise use the resolved remote default branch. Do not merge local `main`, local `master`, or any other local target branch.
2. Inspect local status and diffs before the update. Do not start a target-branch merge with unrelated staged, unstaged, or untracked changes. If existing local changes are intended blocker fixes, commit or otherwise finish that coherent batch first; if they are unrelated or inseparable, stop and ask how to split or scope the work.
3. Fetch the exact remote target branch needed for the update. Determine the repo or PR-preferred update strategy from repository docs, branch policy, PR settings, or sibling practice when available.
4. If policy requires rebase, linear history, force-push, destructive reset, or another history-rewriting update, stop unless the user explicitly authorized that operation. Otherwise, merge the remote target branch into the current source branch without rewriting branch history.
5. If conflicts occur, resolve them inline while preserving current branch intent and incorporating incoming target-branch changes. Do not choose one side wholesale just to remove markers. Read conflict context and enough history from both sides; accept incoming removals of feature flags, dead code, deprecated APIs, or temporary constructs this branch did not introduce, then adapt branch code to the removal.
6. Search for `<<<<<<<`, `=======`, and `>>>>>>>`. Run the most relevant available check for the merge result. If checks expose merge-caused failures, fix them before completing/publishing or report the exact blocker.
7. Stage only resolved/intended files and complete the merge or update commit only after conflict markers are gone and known merge-caused failures are handled.

### Failed required check or CI gate

Use this path when the active blocker is a red required check, CI job, pipeline, merge queue run, synthetic-merge run, test, build, lint, type, schema, deployment, or custom validation.

1. Capture the failing source before editing: check/job name, URL or UUID, latest relevant SHA, command when visible, step name, exit code, failing test/assertion, compiler/linter diagnostic, stack trace, timeout signature, infrastructure signature, and the key log lines. If logs are inaccessible, refresh auth once when appropriate, retry once, then report a tooling blocker.
2. If logs show timeout, worker loss, network failure, dependency outage, rate limit, provider outage, or known nondeterministic failure without branch-caused evidence, classify it as flaky/infrastructure. If provider convention and permissions allow it, perform one safe rerun or bounded wait; otherwise report the required red gate as still merge-blocking. Do not edit code for infrastructure noise.
3. Inspect only the failing seam before editing: failing command or entry point, failing code/config path, relevant state/model, representative caller/consumer, adjacent tests, and 2-3 sibling patterns when available.
4. Reproduce locally with the closest useful command when feasible. If reproduction is CI-only, environment-specific, permission-limited, or too expensive, proceed from captured CI evidence and state the proof gap.
5. Fix the root cause in application code, intended-behavior tests, fixtures, owned config, or owned infrastructure required by this branch. No suppressions, baselines, skipped/deleted/weakened tests, sleeps, broad catches, silent fallbacks, fake success exits, retry wrappers that hide deterministic defects, or code that dodges the checker.
6. Run targeted proof tied to the original diagnostic: the failing test, job-equivalent command, typecheck, lint rule, schema validator, build step, contract script, conflict-marker search, or CI rerun. A broad command such as `quality`, `check`, or `test` is not sufficient by itself unless evidence shows it includes the failing check or diagnostic.
7. Compare the original failing signal with the new passing signal or changed diagnostic. If no code/config/test/doc diff exists and no targeted rerun/status changed, do not say the blocker was fixed; report the current evidence and next exact blocker instead.

### Required review comment or task

Use this path when unresolved comments/tasks are required or likely to block merge.

1. Fetch unresolved comments/tasks once for the latest source SHA. If none exist, mark comment/task blockers absent for that SHA and do not revisit them until after a push or explicit user focus.
2. Prefer current unresolved comments over stale, resolved, outdated, or superseded comments. Keep stale comments only when they still point to code present in the effective diff.
3. Read only the selected comment/task text, referenced lines, surrounding code, relevant PR/base diff hunk, owning interface/state/model, adjacent tests, and 2-3 sibling patterns when available.
4. Classify selected comments/tasks from evidence: `correct`, `unclear`, `subjective`, `wrong`, `YAGNI`, or `already-addressed`.
5. Fix `correct` comments in the smallest coherent batch, starting with correctness, tests, build, API contracts, data safety, or comments that block reliable verification. For `unclear` comments, ask one concise clarifying question and leave them unmodified until answered. Do not appease a reviewer for `subjective`, `wrong`, `YAGNI`, or `already-addressed` comments unless the user explicitly chooses that.
6. Validate with the narrowest targeted check that proves the comment concern. If no automated seam exists, name the manual proof. Do not resolve threads, post replies, or dismiss reviews unless explicitly requested.

### Missing, skipped, canceled, queued, or pending required gate

Use this path when a required gate is not red with a branch-caused diagnostic but still blocks merge.

1. Identify the gate name, latest relevant SHA, provider state, and whether it is required for merge.
2. If provider convention and permissions allow a safe rerun, trigger it once. If the normal path is waiting, use one bounded wait. Do not repeatedly rerun, poll, or refresh without new evidence.
3. If the gate remains missing, skipped, canceled, queued, or pending, report it as still merge-blocking with the exact state and last checked time/SHA.

### Human, policy, or tooling blocker

Use this path for missing required approvals, changes-requested state, draft PR state, unresolved required tasks with no safe code action, Jira issue requirements, deployment/security/compliance/ownership/custom merge checks, permissions, inaccessible logs, unavailable provider APIs, or tool errors.

Report the exact blocker and the human or system action required. Do not try to clear it through unrelated code churn. Do not claim merge-ready while a required provider gate remains unsatisfied.

## Fix constraints

Make the smallest production, test, fixture, documentation, configuration, or owned infrastructure change that truly clears the active blocker. Add or update a targeted test when the blocker identifies a realistic regression and a test seam exists.

Do not broaden scope to unrelated refactors, style churn, optional polish, opportunistic migrations, or failures outside this branch. Do not blindly refetch, rerun, repush, poll, or restart merge queues. Each loop must follow a fix, a target-branch update, a targeted rerun, a bounded wait, a narrowed hypothesis, or new blocker evidence. Stop with a concrete blocker instead of looping on inaccessible logs, missing permissions, unavailable checks, flaky infrastructure, unresolved human review state, or ambiguous intent.

## Publish and refresh

1. Before staging, inspect final local status and intended diff for the active blocker. If staged and unstaged diffs are empty and no target-branch merge/update commit or provider-state change occurred, do not commit or push; return to the active blocker evidence.
2. Stage only intended blocker-fix or target-update changes. If pre-existing unpushed commits or local changes are unrelated to the blocker fixes, stop and ask how to split or scope the publish.
3. Commit with a grounded Conventional Commit subject when a normal fix commit is needed; allow the default merge commit message for a target-branch merge commit when it accurately records the update.
4. Push the current source branch to its configured upstream, or to `origin` with upstream set if none exists and `origin` is the appropriate branch remote.
5. After a push, completed target update, targeted rerun, or bounded wait, refresh one bounded blocker snapshot for the latest relevant SHA. Continue only when the refreshed snapshot identifies a new local-fixable active blocker. Otherwise stop and report the remaining human/policy/tooling/pending/flaky blocker or that no local-fixable blockers remain.

Do not loop solely because the review UI still shows an old comment that the pushed diff already addresses. Do not claim fixed, complete, passing, unblocked, merge-ready, or cleared unless the latest checked source SHA and, when relevant, merge queue or synthetic-merge SHA support that claim.

## Done gate

Before claiming no local-fixable blockers remain, verify or report the blocker for each:

- Target review, source branch, target branch, latest source SHA, and active blocker id were identified.
- Conflicts or target-branch update requirements are gone, handled, or blocked for a stated reason.
- Failed required checks are passing on the latest relevant SHA, superseded by green newer runs, or classified with evidence as stale, flaky/infrastructure, unrelated, or tooling-blocked.
- Required comment/task blockers are addressed, absent for the latest checked SHA, unclear with a question, or classified with evidence as non-actionable.
- Pending/missing/skipped/canceled gates have had at most one safe rerun or bounded wait and are reported if still merge-blocking.
- Required approvals, changes-requested state, draft state, Jira/compliance/custom gates, and permissions are satisfied or reported as human/policy/tooling blockers.
- Any published commit set is coherent and contains only intended blocker fixes and target-branch update commits.
- Targeted proof was run for each fixed batch, or the exact proof gap is named.
- Final status and diff were inspected for accidental files, secrets, debug output, generated noise, and unrelated edits.

## Final

- Target: `<review/branch or blocker>`
- Latest checked SHA: `<source SHA and merge/synthetic SHA when relevant>`
- Active blocker handled: `<type/id -> action/evidence>`
- Changed: `<files or none>`
- Proof: `<targeted checks/results or proof gap>`
- Publish/refresh: `<commit/push/rerun/wait/refreshed status | not needed | blocker>`
- Remaining: `<none local-fixable | exact blocker>`
- Human action required: `<none or exact reviewer/policy/permission action>`
