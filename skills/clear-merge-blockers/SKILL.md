---
name: clear-merge-blockers
description: "Clear PR merge blockers by persistently driving one concrete blocker at a time until provider-proven green CI or an exact human/policy/tooling blocker: failed CI, actionable review comments/tasks, conflicts, branch update requirements, or pending gates."
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Clear merge blockers for `$ARGUMENTS` or the current branch's review. Optimize for the next blocker that can move the PR toward green, not for a full PR audit.

## Authorization

A request to study, review, explain, or plan this skill or a blocker surface authorizes no writes, reruns, waits, or queue actions. A request to clear, fix, resolve, apply, publish, iterate, or keep going authorizes only this workflow's actions: stage intended blocker fixes, create coherent commits, fetch the exact remote target branch, merge it into the current source branch when that is the safe repo-compatible update strategy, push the current branch to its configured upstream or `origin`, perform safe required-check reruns and bounded waits for active current-sha gates when provider convention and permissions allow, and refresh until the terminal state is `green` or an exact human/tooling blocker prevents safe progress.

Do not create branches, open reviews, merge PRs, force-push, reset, discard, stash, rebase a published branch, resolve comment threads, dismiss reviews, approve, post review replies, or merge/restart a merge queue unless explicitly requested. Stop before any write if the current branch is `main`, `master`, or the resolved remote default branch, unless the user explicitly named that branch as the write target.

## Claims

Never say or imply `done`, `cleared`, `unblocked`, `merge-ready`, `all green`, `passing`, or `no blockers` unless the terminal state gate below proves `green` from current provider data. Say `CI is green` only when current-sha CI gates are proven terminal-success, and pair it with the merge-blocker state when non-CI blockers remain. Local checks prove only local behavior, never hosted CI, required gates, mergeability, approvals, or policy.

## Persistence

A clear/fix/iterate request is complete only when provider data proves `green`, or when a human/policy/tooling blocker makes further safe progress impossible. "No local changes left" is not an exit. `needs-local-fix` means fix and publish the active blocker; `waiting` means keep the current gate active and wait/refresh, or perform the one safe rerun allowed for that unchanged gate state. Do not produce a final response while either has a safe next action.

If the host provides an active-goal mechanism and the user asked to keep going until CI is green, use it: complete the goal only at `green`, and block it only on a repeated exact human/tooling blocker or a hard execution limit. Otherwise apply the same contract in the current turn.

A bounded wait is one wait sized to the gate's expected remaining runtime, not a tight poll. If a gate shows no provider progress across refreshes spanning well beyond its normal runtime, treat it as stuck and report `waiting` with the gate, SHA, and elapsed time.

During waits, treat new user input that claims CI is red, green, stale, rerun, or changed as fresh evidence: end the wait as soon as the host allows, refresh the exact PR/check/pipeline, and do not report from data collected before that message.

## Provider reconciliation

Current-sha red wins. If any merge-relevant surface for the latest source SHA reports `FAILED`, `ERROR`, rejected, timed out, or equivalent, the state is `needs-local-fix` until that signal is proven stale, superseded by a newer green run of the same gate, or unrelated with evidence. A queued, running, or missing signal elsewhere does not override it.

When surfaces disagree, query the exact gate by id, build number, or status URL and reconcile by SHA, gate name/key, build number/UUID, and completion timestamps. If logs are unavailable but a current red summary exists, keep it as the active blocker and report the log-access gap; do not downgrade it to `waiting`.

For Bitbucket, collect both PR `_statuses` and the matching pipeline runs for the source branch/SHA. Save the PR with `twg bb prs get <id> --statuses -o json --output-file <pr.json>`; terminal output may be a YAML summary rather than JSON. A PR status such as `FAILED` with a pipeline URL or build number is sufficient red evidence to leave `waiting`; fetch the pipeline detail and logs next when accessible.

## Terminal state gate

Each refresh ends in exactly one state:

- `needs-local-fix`: a current conflict, update requirement, failed check, or correct unresolved review comment has a safe local action.
- `waiting`: a required or user-relevant gate is queued, pending, running, missing, skipped, or canceled, and a bounded wait/refresh or one safe rerun remains.
- `human-blocked`: approval, draft, changes-requested, Jira, compliance, ownership, permission, or policy state requires a human/system action.
- `tooling-blocked`: provider data, logs, the required-gate list, the current SHA, or auth cannot be obtained after the allowed retry/auth step.
- `green`: every merge-relevant gate is current and terminal-success, with nothing red, pending, missing, skipped, canceled, stale, or unknown.

CI state (`green`, `red`, `waiting`, `unknown`) covers current-sha CI gates only. Merge-blocker state is the full state above, including review, policy, mergeability, and permissions; CI can be `green` while merge-blocker state is `human-blocked`.

Before claiming `green`, prove from provider data:

- The review id, source and target branches, source/head SHA, and any merge queue or synthetic-merge SHA.
- Every checked status belongs to the latest of those SHAs.
- Every visible current-sha check, status, pipeline, merge queue run, synthetic-merge run, and custom gate is terminal success, or is non-applicable, stale, or superseded with evidence. If the user asked for all pipelines, include non-required ones.
- Mergeability, update, and conflict state are satisfied.
- Required comments/tasks, approvals, draft and changes-requested state, Jira/compliance/custom gates, and permissions are satisfied or explicitly absent.

If the required-gate set cannot be enumerated, the state is `tooling-blocked`. A red gate is `needs-local-fix` with branch-caused evidence; otherwise classify it as human-blocked, tooling-blocked, or flaky/infrastructure as evidenced.

### CI green helper

Run the bundled helper on the CI snapshot before a final classification. A partial snapshot can prove red; only a complete snapshot can prove green. The helper does not check approvals, policy, or mergeability.

Resolve the skill directory from the loaded `clear-merge-blockers/SKILL.md`, not from the target repository, then run:

```bash
python3 <skill-directory>/scripts/ci_green_gate.py <snapshot.json>
```

If the command fails because the path resolved inside the target repo, correct the path and rerun; do not report the helper as absent.

The snapshot is a JSON object. A saved provider payload works as-is for proving red, but green additionally requires the completeness flags, so add them only after verifying what they assert:

- `head_sha`: the latest source/head SHA (Bitbucket `source.commit.hash` is also read).
- `merge_sha`, `synthetic_sha`, `merge_queue_sha`: include every identified latest alternate SHA that must be green; a gate entry on that SHA is not a substitute for declaring it here.
- `relevant_shas_complete: true`: only after identifying the complete relevant SHA set and declaring every alternate SHA above.
- `provider_snapshot_complete: true`: only after enumerating the provider gate set for those SHAs.
- `checks`, `pipelines`, `statuses`, `gates`, or Bitbucket `_statuses`: entries with `name`, `state` or `conclusion`, `sha` or `commit` when available, and `required` when known. Pipeline detail with `target.commit.hash` is read too.
- `scoped_to_head: true`: only when the provider query was scoped to the latest SHA and gates do not expose SHAs, as with PR `_statuses`.

```json
{"head_sha": "abc1234", "relevant_shas_complete": true, "provider_snapshot_complete": true, "scoped_to_head": true, "statuses": [{"name": "build", "state": "SUCCESSFUL", "required": true}]}
```

Exit 0 means the snapshot is complete and every gate for the latest SHA is green. Nonzero means the state is not `green`; use the output as blocker evidence. If the helper truly cannot run, the state is `tooling-blocked` for any green claim; apply the same rules manually for non-green states and say why the helper added no evidence.

## One active blocker

Every iteration names exactly one active blocker before deep investigation or editing:

- Type: `conflict`, `update-required`, `failed-check`, `comment-task`, `missing-or-pending-gate`, `human-policy`, or `tooling-blocked`.
- Id: PR id plus check/job/pipeline UUID, comment/task id, conflict path, gate name, or mergeability state.
- Latest relevant SHA, including any merge queue or synthetic-merge SHA.
- Original evidence: the exact provider status, diagnostic, comment text, conflict file, or policy message.
- Next action: fix, merge target, rerun, bounded wait, ask, or report a non-local blocker.

No blocker id, no work. No original evidence, no fix. No diff, target update, provider-state change, successful rerun, or new evidence, no claim of progress. Refresh the full PR only after a code/status change, a completed targeted check or rerun, or when evidence became stale.

## Blocker snapshot

1. If `$ARGUMENTS` names a PR, check, job, run, comment, task, file, conflict, update warning, merge queue result, pasted failure, or gate, make it the active blocker and inspect only enough surrounding state to act safely.
2. Otherwise collect one bounded snapshot: repository, current branch, upstream, remote default branch, working tree status, unpushed commits, PR id, source and target branches, latest source SHA, mergeability/conflict/update state, all visible current-sha checks/pipelines/statuses/custom gates with required flags when known, unresolved review comments and required tasks, draft/approval/changes-requested state, and Jira/compliance/custom gate messages.
3. Before any local write, resolve the review source repository, branch, head SHA, and the remote and destination ref that would receive the push (the configured upstream, otherwise `origin` and the review source branch). Prove that the selected remote has exactly one push URL identifying the review source repository, the destination ref is the review source branch, the current branch is the source branch, and the provider head SHA equals `HEAD` or is an ancestor of `HEAD` with only coherent local commits ahead. Refresh the source ref if needed. If any of this cannot be proven, stop with the exact checkout blocker; this skill does not authorize switching branches.
4. Do not read the full PR diff, all comments, or all logs during the snapshot. Read code, diffs, and logs only for the active blocker.
5. Prefer the latest signal for the latest SHAs. Ignore stale, resolved, outdated, superseded, or other-branch signals unless they still describe code or required state in the active blocker.
6. If there is no target review, no accessible blocker data, or no coherent way to separate blocker fixes from unrelated local changes, stop and report it. With no local-fixable blocker but a non-terminal current gate, use the waiting playbook; with a current red surface, use the failed-check playbook.

Choose the first blocker in this order unless the user named a narrower focus:

1. Conflicts or branch-update-required states.
2. Failed required or merge-blocking checks, including merge queue or synthetic-merge failures.
3. Actionable unresolved review comments, from people or bots, and required tasks.
4. Missing, skipped, canceled, queued, or pending required gates that can be rerun or waited on.
5. Human review, draft, Jira, compliance, policy, permission, or tooling blockers.

Once a blocker class is absent for the current source SHA, do not revisit it until a push, rerun, target update, completed wait, or explicit user focus. Recheck comments after every wait, because reviewers and bots comment while CI runs.

## Playbooks

### Conflict or update required

Use when the branch is conflicted, behind the target, blocked by an update-required policy, or failing only on a merge queue or synthetic merge because of target-branch integration.

1. Identify the exact remote target branch from the PR, else the resolved remote default branch. Never merge a local `main`, `master`, or other local target branch.
2. Do not start a merge with unrelated staged, unstaged, or untracked changes. Finish an intended blocker-fix batch first; if changes are unrelated or inseparable, ask how to split them.
3. Fetch the exact remote target branch. Take the update strategy from repository docs, branch policy, PR settings, or sibling practice.
4. If policy requires rebase, linear history, force-push, or another history rewrite, stop unless the user authorized it. Otherwise merge the remote target branch into the source branch.
5. Resolve conflicts preserving branch intent and incorporating incoming changes; never pick a side wholesale to remove markers. Read both sides' history (`git log --merge -p -- <file>`). Accept incoming removals of flags, dead code, deprecated APIs, or temporary constructs this branch did not introduce, and adapt branch code.
6. Search the unmerged files for `<<<<<<<`, `=======`, and `>>>>>>>`, then run or reuse the most relevant check for the merge result and fix merge-caused failures.
7. Stage the resolutions, require `git ls-files -u` to be empty, and commit the merge only after markers and known merge-caused failures are gone.

### Failed check or CI gate

Use when the active blocker is a red check, job, pipeline, merge queue run, synthetic-merge run, test, build, lint, type, schema, deployment, or custom validation.

1. Capture the failing source before editing: gate name, URL or UUID, SHA, command, step, exit code, failing test or diagnostic, stack trace, timeout or infrastructure signature, and key log lines. If logs are inaccessible, refresh auth once when appropriate, retry once, then report a tooling blocker.
2. Timeouts, worker loss, network or dependency outages, rate limits, or known nondeterminism without branch-caused evidence are flaky/infrastructure. Perform one safe rerun or bounded wait when convention and permissions allow; otherwise report the red gate as still blocking. Do not edit code for infrastructure noise.
3. Inspect only the failing seam: the command or entry point, the failing code/config path, relevant state, a representative caller, adjacent tests, and two or three sibling patterns.
4. Reproduce locally with the closest command when feasible; otherwise proceed from CI evidence and state the gap.
5. Fix the root cause in application code, intended-behavior tests, fixtures, or owned config/infrastructure. No suppressions, baselines, skipped/deleted/weakened tests, sleeps, broad catches, silent fallbacks, fake success exits, retry wrappers that hide deterministic defects, or checker dodges.
6. Prove the fix with the check tied to the original diagnostic. Do not rerun a required check that is already green for the latest SHA. A broad `check` or `test` command counts only when evidence shows it includes the failing diagnostic.
7. Compare the original failing signal with the new passing or changed one. Without a diff, a changed status, or valid reused proof, do not say the blocker was fixed.

### Review comment or task

Apply the `address-comments` classification and fix rules, but publish through this loop rather than running a second one.


1. Fetch unresolved comments/tasks once for the latest source SHA. If none exist, mark the class absent for that SHA.
2. Prefer current unresolved comments; keep stale ones only when they still point to code in the effective diff.
3. Read only the selected comment, referenced lines, surrounding code, the relevant diff hunk, owning interface, adjacent tests, and sibling patterns.
4. Classify from evidence: `correct`, `unclear`, `subjective`, `wrong`, `YAGNI`, or `already-addressed`.
5. Fix `correct` comments in the smallest coherent batch, starting with correctness, tests, build, API contracts, and data safety. Ask one concise question for `unclear` ones and leave them unmodified. Do not act on the other classes unless the user chooses to.
6. Prove the fix with the narrowest check, or name the manual proof. Do not resolve threads, reply, or dismiss reviews unless asked.

### Missing, skipped, canceled, queued, or pending gate

1. Identify the gate, SHA, provider state, and whether it is required.
2. Rerun a missing, skipped, or canceled gate once for that unchanged state when convention and permissions allow; otherwise wait.
3. Keep the gate active while another safe refresh is possible. Never rerun the same unchanged state twice; each poll cites the gate, SHA, elapsed time, and current state.
4. After each wait, refresh every surface that contributed to the active blocker and reconcile red against pending.
5. Stop at `waiting` only for a status-only request, an explicit user/harness limit, a stuck gate, blocked provider access, or a state that needs human/system action.

### Human, policy, or tooling blocker

Use for missing approvals, changes-requested or draft state, tasks with no safe code action, Jira requirements, deployment/security/compliance/ownership/custom checks, permissions, inaccessible logs, unavailable provider APIs, or tool errors. Report the exact blocker and the action required. Do not churn code to clear it.

## Fix constraints

Make the smallest change that truly clears the active blocker, and add a targeted test when the blocker names a realistic regression and a seam exists. No unrelated refactors, style churn, opportunistic migrations, or fixes for failures outside this branch. Each loop must follow a fix, target update, rerun, bounded wait, narrowed hypothesis, or new evidence; never blindly refetch, rerun, repush, poll, or restart merge queues. Report a hard blocker rather than looping on inaccessible logs, missing permissions, flaky infrastructure, human review, or ambiguous intent, but do not call an actively progressing gate a hard blocker.

## Publish and refresh

1. Inspect final status and the intended diff. With no diff and no merge commit or provider-state change, do not commit or push.
2. Stage only blocker-fix or target-update changes. If unpushed commits or local changes are unrelated, ask how to split the publish.
3. Commit with a grounded Conventional Commit subject; the default merge message is fine for a target-branch merge.
4. Push with `git push <selected-remote> HEAD:<review-source-branch>`, adding `-u` only when no upstream exists.
5. After a push, update, rerun, or wait, refresh one bounded snapshot for the latest SHAs across every previously observed surface and route the result through the state machine. Never collapse `waiting`, red non-required CI, or unknown data into "no local-fixable blockers" without naming the remaining state.

Do not loop because the review UI still shows a comment the pushed diff already addresses.

## Final

Finish only at `green`, `human-blocked`, `tooling-blocked`, or `waiting` under a stop condition above. Lead with CI state and merge-blocker state together. Report the target, latest checked SHAs, remaining gates or human actions, helper and provider evidence, fixes and proof, comments addressed or left with their reasons, commit/push/rerun results, and evidence gaps. Inspect final status and diff for unrelated changes. Keep it concise.
