---
name: clear-merge-blockers
description: "Clear PR merge blockers by persistently driving one concrete blocker at a time until provider-proven green CI or an exact human/policy/tooling blocker: failed CI, actionable review comments/tasks, conflicts, branch update requirements, or pending gates."
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Clear merge blockers for `target`, `$ARGUMENTS`, or the current branch's review. Optimize for the next blocker that can move the PR toward green, not for a full PR audit.

A request to study, review, explain, or plan this skill or a blocker surface does not authorize edits, commits, merges, pushes, PR state changes, check reruns, waits, or queue actions. A request to clear, fix, resolve, apply, publish, iterate, or keep going on merge blockers is explicit authorization to perform only the actions required by this finish-the-review workflow: stage intended blocker fixes, create coherent commits, fetch the exact remote target branch, merge the remote target branch into the current source branch when that is the safe repo-compatible update strategy, push the current branch to its configured upstream or `origin`, perform safe required-check reruns and bounded wait/refresh cycles for active current-sha gates when provider convention and permissions make that appropriate, refresh the blocker snapshot, and continue until the terminal provider state is `green` or an exact human/tooling blocker prevents further safe progress.

Do not create branches, open reviews, merge PRs, force-push, reset, discard, stash, rebase a published branch, resolve comment threads, dismiss reviews, approve, post review replies, or merge/restart a merge queue unless explicitly requested. Stop before any write if the current branch is `main`, `master`, or the resolved remote default branch, unless the user explicitly named that branch as the write target.

Never say or imply `done`, `complete`, `cleared`, `unblocked`, `merge-ready`, `CI is green`, `all green`, `passing`, or `no blockers` unless the terminal provider state gate below is satisfied with current hard evidence. Local checks prove only local behavior; they never prove hosted CI, required gates, mergeability, approvals, or policy state.

## Persistence contract

This skill is goal-like: a clear/fix/iterate request is not complete when there are "no local changes left" or "no local-fixable blockers right now." It is complete only when provider data proves `green`, or when a human/policy/tooling blocker makes further safe progress impossible.

The state machine controls the next action; it is not an exit condition. `needs-local-fix` means fix and publish the active blocker. `waiting` means keep the current gate as the active blocker and wait/refresh, or perform the one safe rerun allowed for that unchanged gate state, until the gate becomes green, red with a diagnostic, human-blocked, or tooling-blocked. Do not produce a final response while `needs-local-fix` or `waiting` still has a safe next action.

If the host provides an active-goal mechanism and the invocation explicitly asks to keep going until CI is green, use that mechanism according to the host rules. Mark the goal complete only after `green`; mark it blocked only for a repeated exact human/tooling blocker or another hard execution limit that prevents another safe refresh. If no goal mechanism is available, emulate the same contract in the current turn.

During waits, treat new user input as fresh blocker evidence when it claims CI is red, green, stale, rerun, changed, or otherwise contradicts the last provider snapshot. Abort or finish the current bounded sleep as soon as the host allows, immediately refresh the exact PR/check/pipeline, and do not produce a final report from provider data collected before that user message.

## Provider reconciliation

Current-sha red wins. If any merge-relevant provider surface for the latest source SHA reports `FAILED`, `ERROR`, `STOPPED`, rejected, timed out, or an equivalent red state, classify the workflow as `needs-local-fix` until that red signal is proven stale, superseded by a newer green run for the same gate, or unrelated with evidence. Do not let a queued/running/missing signal from another surface override a current red PR status, commit status, pipeline result, merge queue result, or check run.

When provider surfaces disagree, query the exact gate by id/build number/status URL and reconcile by SHA, gate name/key, build number/UUID, and `updated_on`, `completed_on`, or equivalent timestamps. If exact logs are unavailable but a current red status summary exists, keep the red status as the active blocker and report the log-access proof gap; do not downgrade it to `waiting`.

For Bitbucket, collect both PR `_statuses` and matching `bb pipeline query` runs for the source branch/SHA. Pass the raw `twg -o json bb prs get <id> --statuses` output to the bundled helper when available; it understands `_statuses` and `source.commit.hash`. The helper also understands raw `twg -o json bb pipeline get --pipeline <number>` detail objects with `target.commit.hash`. A PR status such as `FAILED` with a pipeline URL/build number is sufficient red evidence to leave `waiting`; fetch the pipeline detail/logs next when accessible.

## Terminal provider state gate

Treat the workflow as a persistent state machine. Each refresh must end in exactly one state:

- `needs-local-fix`: a current conflict, update requirement, failed check, or correct required comment has a safe local action.
- `waiting`: a required or user-relevant gate is queued, pending, in progress, missing, skipped, or canceled; when safe progress remains, the next action is a bounded wait/refresh or one safe rerun for the unchanged gate state.
- `human-blocked`: approval, draft, changes-requested, Jira, compliance, ownership, permission, or policy state requires a human/system action.
- `tooling-blocked`: provider data, logs, required-gate list, current SHA, or auth cannot be obtained after the allowed retry/auth step.
- `green`: every merge-relevant provider gate is explicitly current and terminal-success, with no red, pending, missing, skipped, canceled, stale, or unknown gate.

Only `green` permits a successful final claim that CI is green or the PR has no blockers. `needs-local-fix` and `waiting` are loop states while a safe next action exists. `human-blocked` and `tooling-blocked` are hard exit states only after the required evidence, retry, auth, or clarification step has been exhausted.

Before claiming `green`, prove all of the following from provider data, not from inference:

- The target review id, source branch, target branch, source/head SHA, and any merge queue or synthetic-merge SHA were identified.
- The checked provider status belongs to the latest source/head SHA and, when applicable, the latest merge queue or synthetic-merge SHA.
- Every visible current-sha CI check, status, pipeline, merge queue run, synthetic-merge run, and custom gate is terminal success, or is explicitly non-applicable/stale/superseded with evidence. If the user asked for all pipelines, include non-required visible pipelines too.
- No required or merge-blocking provider gate is failed, errored, canceled, skipped, missing, queued, pending, running, stale, or unknown.
- Mergeability/update/conflict state is satisfied.
- Required comments/tasks, approvals, draft state, changes-requested state, Jira/compliance/custom policy gates, and permissions are satisfied or explicitly absent.

If the complete required-gate set cannot be enumerated, the state is `tooling-blocked`, not `green`. If any gate is pending/running/missing/skipped/canceled, the state is `waiting`, not `green`. If any gate is red, the state is `needs-local-fix` when branch-caused evidence exists, otherwise `human-blocked`, `tooling-blocked`, or flaky/infrastructure as evidenced.

### CI green helper

For any final CI/provider classification, run the bundled helper after collecting a complete provider snapshot unless provider access itself is the blocker.

Use the helper from this skill bundle, not from the target repository. In this checkout, run:

```bash
python3 /Users/asumer/src/config/.agents/skills/clear-merge-blockers/scripts/ci_green_gate.py <snapshot.json>
```

If the skill is installed elsewhere, locate `ci_green_gate.py` beside the loaded `clear-merge-blockers/SKILL.md`. Do not run `<target-repo>/scripts/ci_green_gate.py` unless that file is explicitly the bundled helper. If helper execution fails because the path was resolved relative to the target repo, correct the path and rerun; do not report the helper as absent or finish from that failed invocation.

Normalize the snapshot to JSON with:

- `head_sha`: the latest source/head SHA.
- `provider_snapshot_complete: true`: set only after enumerating the provider gate set for that SHA.
- `checks`, `pipelines`, `statuses`, or `gates`: entries include `name`, `state` or `conclusion`, `sha` or `commit` when available, and `required` when known.
- `scoped_to_head: true`: set only when the provider query itself was scoped to the latest SHA and individual gates do not expose SHAs.

The helper exits 0 only when the snapshot is complete and every included gate for the latest SHA is terminal green. A nonzero exit means the final state is not `green`; use the helper output as blocker evidence. If the bundled helper truly cannot be found or run after locating the skill resources, the state is `tooling-blocked` for a green/done claim. For non-green provider states, manually apply the same state rules and say why the helper could not add evidence.

## Operating model: one active blocker

Every iteration must have exactly one active blocker before deep investigation or editing:

- Type: `conflict`, `update-required`, `failed-check`, `comment-task`, `missing-or-pending-gate`, `human-policy`, or `tooling-blocked`.
- Id: PR/review id plus check/job/pipeline UUID, comment/task id, conflict path, gate name, or mergeability state.
- Latest relevant SHA: source/head SHA and, when applicable, merge queue or synthetic-merge SHA.
- Original evidence: the exact provider status, failing diagnostic, comment text, conflict marker/file, or policy message.
- Next local action: fix, merge target, rerun, bounded wait/refresh, ask a question, or report a non-local blocker.

No blocker id, no work. No original evidence, no fix. No diff, completed target update, provider-state change, successful rerun, or new blocker evidence, no claim of progress. Do not perform another full PR refresh unless a code/status change was made, a targeted check/rerun completed, or the previous evidence became stale or insufficient.

## Fast blocker snapshot

1. If `target`, `$ARGUMENTS`, or `focus` names a PR/review, check, job, run, comment, task, file, conflict, branch-update warning, merge queue result, pasted failure output, or required gate, make that the active blocker and inspect only enough surrounding review state to act safely.
2. With no supplied target, collect one bounded blocker snapshot: repository, current branch, upstream, remote default branch, working tree status, unpushed commits, PR/review id, source branch, target branch, latest source/head SHA, mergeability/conflict or update-required state, all visible current-sha CI checks/pipelines/statuses/custom gates and which are required when known, unresolved required comments/tasks, draft/approval/changes-requested state, and Jira/compliance/custom gate messages when available.
3. Do not read the full PR diff, all untracked files, all comments, or all check logs during the snapshot. Read code, diffs, and logs only for the selected active blocker.
4. Prefer the latest signal for the latest source/head SHA and, when the provider uses one, the latest merge queue or synthetic-merge SHA. Ignore stale, resolved, outdated, superseded, or other-branch signals unless they still describe code or required state present in the selected blocker.
5. If there is no target review, no accessible blocker data, or no coherent way to separate blocker fixes from unrelated local changes, stop and report the exact blocker. If there is no actionable local-fixable blocker but a current required or user-relevant gate is not terminal success, enter the waiting-gate playbook instead of stopping. If any current-sha surface is red, enter the failed-check playbook instead of waiting. Do not invent changes.

Choose the first local-fixable blocker in this order unless the user named a narrower focus:

1. Conflicts or branch-update-required states.
2. Failed required or merge-blocking checks, including merge queue or synthetic-merge failures.
3. Required unresolved review comments or tasks.
4. Missing, skipped, canceled, queued, or pending required gates that can be safely rerun or waited on.
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
6. Search for `<<<<<<<`, `=======`, and `>>>>>>>`. Run or reuse fresh proof for the most relevant available check for the merge result under the proof policy. If checks expose merge-caused failures, fix them before completing/publishing or report the exact blocker.
7. Stage only resolved/intended files and complete the merge or update commit only after conflict markers are gone and known merge-caused failures are handled.

### Failed required check or CI gate

Use this path when the active blocker is a red required check, CI job, pipeline, merge queue run, synthetic-merge run, test, build, lint, type, schema, deployment, or custom validation.

1. Capture the failing source before editing: check/job name, URL or UUID, latest relevant SHA, command when visible, step name, exit code, failing test/assertion, compiler/linter diagnostic, stack trace, timeout signature, infrastructure signature, and the key log lines. If logs are inaccessible, refresh auth once when appropriate, retry once, then report a tooling blocker.
2. If logs show timeout, worker loss, network failure, dependency outage, rate limit, provider outage, or known nondeterministic failure without branch-caused evidence, classify it as flaky/infrastructure. If provider convention and permissions allow it, perform one safe rerun or bounded wait and refresh into the persistent state machine; otherwise report the required red gate as still merge-blocking. Do not edit code for infrastructure noise.
3. Inspect only the failing seam before editing: failing command or entry point, failing code/config path, relevant state/model, representative caller/consumer, adjacent tests, and 2-3 sibling patterns when available.
4. Reproduce locally with the closest useful command when feasible. If reproduction is CI-only, environment-specific, permission-limited, or too expensive, proceed from captured CI evidence and state the proof gap.
5. Fix the root cause in application code, intended-behavior tests, fixtures, owned config, or owned infrastructure required by this branch. No suppressions, baselines, skipped/deleted/weakened tests, sleeps, broad catches, silent fallbacks, fake success exits, retry wrappers that hide deterministic defects, or code that dodges the checker.
6. Run or reuse targeted proof tied to the original diagnostic: the failing test, job-equivalent command, typecheck, lint rule, schema validator, build step, contract script, conflict-marker search, or CI rerun. If a required check is already green for the latest relevant SHA, do not rerun it. A broad command such as `quality`, `check`, or `test` is not sufficient by itself unless evidence shows it includes the failing check or diagnostic.
7. Compare the original failing signal with the new or reused passing signal or changed diagnostic. If no code/config/test/doc diff exists, no targeted rerun/status changed, and no valid reused proof applies, do not say the blocker was fixed; report the current evidence and next exact blocker instead.

### Required review comment or task

Use this path when unresolved comments/tasks are required or likely to block merge.

1. Fetch unresolved comments/tasks once for the latest source SHA. If none exist, mark comment/task blockers absent for that SHA and do not revisit them until after a push or explicit user focus.
2. Prefer current unresolved comments over stale, resolved, outdated, or superseded comments. Keep stale comments only when they still point to code present in the effective diff.
3. Read only the selected comment/task text, referenced lines, surrounding code, relevant PR/base diff hunk, owning interface/state/model, adjacent tests, and 2-3 sibling patterns when available.
4. Classify selected comments/tasks from evidence: `correct`, `unclear`, `subjective`, `wrong`, `YAGNI`, or `already-addressed`.
5. Fix `correct` comments in the smallest coherent batch, starting with correctness, tests, build, API contracts, data safety, or comments that block reliable verification. For `unclear` comments, ask one concise clarifying question and leave them unmodified until answered. Do not appease a reviewer for `subjective`, `wrong`, `YAGNI`, or `already-addressed` comments unless the user explicitly chooses that.
6. Validate with the narrowest targeted check that proves the comment concern, or reuse fresh proof when valid. If no automated seam exists, name the manual proof. Do not resolve threads, post replies, or dismiss reviews unless explicitly requested.

### Missing, skipped, canceled, queued, or pending required gate

Use this path when a required gate is not red with a branch-caused diagnostic but still blocks merge.

1. Identify the gate name, latest relevant SHA, provider state, and whether it is required for merge.
2. If provider convention and permissions allow a safe rerun for a missing, skipped, or canceled gate, trigger it once for that unchanged gate state. If the normal path is waiting, use a bounded wait/refresh cycle.
3. Keep the gate as the active blocker while it is missing, skipped, canceled, queued, pending, or running and another safe refresh is possible. Do not repeatedly rerun the same unchanged gate state; repeated polling must cite the gate, SHA, elapsed time or checked time, and current provider state.
4. After every wait, refresh all provider surfaces that contributed to the active blocker, not only the one that was pending before the wait. Reconcile red-vs-pending disagreements with the provider reconciliation rules.
5. Stop with `waiting` only when the user requested status-only, an explicit user/harness limit prevents another wait, provider access/auth is blocked, or the provider state proves no further safe progress is possible without human/system action. Otherwise continue until the gate becomes green, red with a diagnostic, `human-blocked`, or `tooling-blocked`.

### Human, policy, or tooling blocker

Use this path for missing required approvals, changes-requested state, draft PR state, unresolved required tasks with no safe code action, Jira issue requirements, deployment/security/compliance/ownership/custom merge checks, permissions, inaccessible logs, unavailable provider APIs, or tool errors.

Report the exact blocker and the human or system action required. Do not try to clear it through unrelated code churn. Do not claim merge-ready while a required provider gate remains unsatisfied.

## Fix constraints

Make the smallest production, test, fixture, documentation, configuration, or owned infrastructure change that truly clears the active blocker. Add or update a targeted test when the blocker identifies a realistic regression and a test seam exists.

Do not broaden scope to unrelated refactors, style churn, optional polish, opportunistic migrations, or failures outside this branch. Do not blindly refetch, rerun, repush, poll, or restart merge queues. Each loop must follow a fix, a target-branch update, a targeted rerun, a bounded wait, a narrowed hypothesis, or new blocker evidence. Report a concrete hard blocker instead of looping on inaccessible logs, missing permissions, unavailable checks, flaky infrastructure, unresolved human review state, or ambiguous intent. Do not downgrade an actively progressing current-sha provider gate into a hard blocker just because there are no local code edits left.

## Publish and refresh

1. Before staging, inspect final local status and intended diff for the active blocker. If staged and unstaged diffs are empty and no target-branch merge/update commit or provider-state change occurred, do not commit or push; return to the active blocker evidence.
2. Stage only intended blocker-fix or target-update changes. If pre-existing unpushed commits or local changes are unrelated to the blocker fixes, stop and ask how to split or scope the publish.
3. Commit with a grounded Conventional Commit subject when a normal fix commit is needed; allow the default merge commit message for a target-branch merge commit when it accurately records the update.
4. Push the current source branch to its configured upstream, or to `origin` with upstream set if none exists and `origin` is the appropriate branch remote.
5. After a push, completed target update, targeted rerun, or bounded wait, refresh one bounded blocker snapshot for the latest relevant SHA across all previously observed provider surfaces. If the refreshed snapshot identifies any current-sha red gate, make it active as a failed-check blocker and continue. If it identifies another local-fixable blocker, make it active and continue. If it is `waiting`, keep the current gate as active and continue the waiting-gate playbook while another safe wait/refresh is possible. If it is `green`, run the done gate and finish. If it is `human-blocked` or `tooling-blocked`, stop and report that exact state. Do not collapse `waiting`, red non-required CI, or unknown provider data into "no local-fixable blockers" without naming and continuing or reporting the remaining provider state.

Do not loop solely because the review UI still shows an old comment that the pushed diff already addresses. Do not claim fixed, complete, passing, unblocked, merge-ready, green, or cleared unless the latest checked source SHA and, when relevant, merge queue or synthetic-merge SHA support that claim through the terminal provider state gate.

## Done gate

Before claiming `green`, verify or report the blocker for each:

- Target review, source branch, target branch, latest source SHA, and active blocker id were identified.
- Conflicts or target-branch update requirements are gone, handled, or blocked for a stated reason.
- Failed required checks are passing on the latest relevant SHA, superseded by green newer runs, or classified with evidence as stale, flaky/infrastructure, unrelated, or tooling-blocked.
- Every visible current-sha CI pipeline/check/status/custom gate was enumerated; any red, pending, missing, skipped, canceled, stale, or unknown gate is reported as remaining and prevents a green/done claim.
- Required comment/task blockers are addressed, absent for the latest checked SHA, unclear with a question, or classified with evidence as non-actionable.
- Pending/missing/skipped/canceled gates have had at most one safe rerun per unchanged gate state, have been followed through bounded wait/refresh cycles while safe progress remained, and are reported only if still merge-blocking because of an explicit limit, human blocker, or tooling blocker.
- Required approvals, changes-requested state, draft state, Jira/compliance/custom gates, and permissions are satisfied or reported as human/policy/tooling blockers.
- Any published commit set is coherent and contains only intended blocker fixes and target-branch update commits.
- Targeted proof was run or reused for each fixed batch, or the exact proof gap is named.
- Final status and diff were inspected for accidental files, secrets, debug output, generated noise, and unrelated edits.

## Final

Produce this final report only when the state is `green`, `human-blocked`, `tooling-blocked`, or `waiting` with an explicit user/harness limit that prevents another safe refresh. Do not produce it for `needs-local-fix`, or for `waiting` when another safe wait/refresh is available.

If the report is not `green`, explicitly say `not complete`. Never use a final report to imply the merge-blocker goal is complete when any current-sha provider gate is red, pending, missing, skipped, canceled, stale, or unknown.

- Target: `<review/branch or blocker>`
- Latest checked SHA: `<source SHA and merge/synthetic SHA when relevant>`
- Terminal provider state: `<green | needs-local-fix | waiting | human-blocked | tooling-blocked, with evidence>`
- CI/provider gates: `<all current-sha gates green | exact red/pending/missing/unknown gates>`
- Active blocker handled: `<type/id -> action/evidence>`
- Changed: `<files or none>`
- Proof: `<targeted checks/results, reused proof, or proof gap>`
- Publish/refresh: `<commit/push/rerun/wait/refreshed status | not needed | blocker>`
- Remaining: `<none only when terminal provider state is green | exact blocker/state>`
- Human action required: `<none or exact reviewer/policy/permission action>`
