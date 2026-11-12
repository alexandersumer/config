---
name: clear-merge-blockers
description: "Clear PR merge blockers by persistently driving one concrete blocker at a time until provider-proven green CI or an exact human/policy/tooling blocker: failed CI, actionable review comments/tasks, conflicts, branch update requirements, or pending gates."
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Clear merge blockers for `$ARGUMENTS` or the current branch's review. Work the next blocker that moves the PR toward green; this is not a full PR audit.

## Authorization

A request to study, review, explain, or plan authorizes no writes, reruns, waits, or queue actions. A request to clear, fix, resolve, apply, publish, iterate, or keep going authorizes only: staging blocker fixes, coherent commits, fetching the exact remote target branch and merging it into the source branch when that is the repo-compatible update strategy, pushing to the configured upstream or `origin`, safe required-check reruns and bounded waits on current-sha gates when convention and permissions allow, and refreshing until `green` or an exact human/tooling blocker.

Do not create branches, open reviews, merge PRs, force-push, reset, discard, stash, rebase a published branch, resolve threads, dismiss reviews, approve, post replies, or merge/restart a merge queue unless explicitly requested. Stop before any write on `main`, `master`, or the resolved remote default branch unless the user named it as the write target.

## Claims and persistence

Never say or imply `done`, `cleared`, `unblocked`, `merge-ready`, `all green`, `passing`, or `no blockers` unless the terminal state gate proves `green` from current provider data. `CI is green` requires current-sha CI gates proven terminal-success; pair it with the merge-blocker state when non-CI blockers remain. Local checks never prove hosted CI, required gates, mergeability, approvals, or policy.

A clear/fix/iterate request ends only at provider-proven `green` or when a human/policy/tooling blocker makes safe progress impossible. "No local changes left" is not an exit. Do not give a final response while `needs-local-fix` or `waiting` has a safe next action. If the host has an active-goal mechanism and the user asked to keep going until green, complete it only at `green` and block it only on a repeated exact blocker or a hard execution limit.

A bounded wait is sized to the gate's expected remaining runtime, not a tight poll. A gate with no provider progress well beyond its normal runtime is stuck: report `waiting` with gate, SHA, and elapsed time. New user input claiming CI changed is fresh evidence: end the wait, refresh the exact gate, and do not report from older data.

## Provider reconciliation

Current-sha red wins. Any merge-relevant `FAILED`, `ERROR`, rejected, or timed-out signal for the latest source SHA means `needs-local-fix` until proven stale, superseded by a newer green run of the same gate, or unrelated with evidence; pending or missing signals elsewhere do not override it. When surfaces disagree, query the exact gate and reconcile by SHA, gate name/key, build number/UUID, and completion time. If logs are unavailable but a current red summary exists, keep it as the active blocker and report the log-access gap.

For Bitbucket, collect both PR `_statuses` and the pipeline runs for the source branch/SHA. Save the PR with `twg bb prs get <id> --statuses -o json --output-file <pr.json>`; terminal output may be a YAML summary rather than JSON. A `FAILED` status with a pipeline URL or build number is red evidence; fetch pipeline detail and logs next.

## Terminal state gate

Each refresh ends in exactly one state:

- `needs-local-fix`: a current conflict, update requirement, failed check, or correct unresolved review comment has a safe local action.
- `waiting`: a required or user-relevant gate is queued, pending, running, missing, skipped, or canceled, and a bounded wait/refresh or one safe rerun remains.
- `human-blocked`: approval, draft, changes-requested, Jira, compliance, ownership, permission, or policy needs human/system action.
- `tooling-blocked`: provider data, logs, the required-gate list, the current SHA, or auth cannot be obtained after the allowed retry/auth step.
- `green`: every merge-relevant gate is current and terminal-success, with nothing red, pending, missing, skipped, canceled, stale, or unknown.

CI state (`green`, `red`, `waiting`, `unknown`) covers current-sha CI gates only; CI can be `green` while merge-blocker state is `human-blocked`.

Before claiming `green`, prove from provider data: the review id, source/target branches, source SHA, and any merge queue or synthetic-merge SHA; that every checked status belongs to the latest of those SHAs; that every visible current-sha check, pipeline, merge queue run, synthetic-merge run, and custom gate is terminal success or evidenced non-applicable/superseded (include non-required ones if the user asked for all pipelines); that mergeability, update, and conflict state are satisfied; and that required comments/tasks, approvals, draft/changes-requested, Jira/compliance/custom gates, and permissions are satisfied or explicitly absent. If the required-gate set cannot be enumerated, the state is `tooling-blocked`.

### CI green helper

Run the bundled helper on the CI snapshot before a final classification. A partial snapshot can prove red; only a complete one can prove green. It does not check approvals, policy, or mergeability. Resolve the skill directory from the loaded `clear-merge-blockers/SKILL.md`, not the target repository:

```bash
python3 <skill-directory>/scripts/ci_green_gate.py <snapshot.json>
```

If the path resolved inside the target repo, fix the path and rerun; do not report the helper as absent.

The snapshot is a JSON object. A saved provider payload works as-is for proving red; add the completeness flags only after verifying what they assert:

- `head_sha`: the latest source SHA (Bitbucket `source.commit.hash` is also read).
- `merge_sha`, `synthetic_sha`, `merge_queue_sha`: include every identified latest alternate SHA that must be green; a gate entry on that SHA is not a substitute for declaring it here.
- `relevant_shas_complete: true`: only after identifying the complete relevant SHA set and declaring every alternate SHA above.
- `provider_snapshot_complete: true`: only after enumerating the provider gate set for those SHAs.
- `checks`, `pipelines`, `statuses`, `gates`, or Bitbucket `_statuses`: entries with `name`, `state` or `conclusion`, `sha` or `commit` when available, and `required` when known. Pipeline `target.commit.hash` is read too.
- `scoped_to_head: true`: only when the query was scoped to the latest SHA and gates expose no SHAs, as with PR `_statuses`.

```json
{"head_sha": "abc1234", "relevant_shas_complete": true, "provider_snapshot_complete": true, "scoped_to_head": true, "statuses": [{"name": "build", "state": "SUCCESSFUL", "required": true}]}
```

Exit 0 means complete and green for the latest SHA; nonzero output is blocker evidence. If the helper truly cannot run, any green claim is `tooling-blocked`; say why it added no evidence.

## One active blocker

Before deep investigation or editing, name exactly one active blocker: type (`conflict`, `update-required`, `failed-check`, `comment-task`, `missing-or-pending-gate`, `human-policy`, `tooling-blocked`), id (PR plus check/pipeline UUID, comment/task id, conflict path, or gate), latest relevant SHA(s), the original provider evidence, and the next action. No blocker id, no work. No original evidence, no fix. No diff, target update, provider-state change, successful rerun, or new evidence, no claim of progress.

## Blocker snapshot

1. If `$ARGUMENTS` names a PR, check, comment, file, conflict, pasted failure, or gate, make it the active blocker and inspect only enough surrounding state to act safely.
2. Otherwise take one bounded snapshot: branch, upstream, remote default, working tree, unpushed commits, PR id, source/target branches, latest source SHA, mergeability/conflict/update state, current-sha checks and gates with required flags, unresolved comments and tasks, approval/draft state, and Jira/compliance messages. Read code, diffs, and logs only for the active blocker.
3. Before any local write, resolve the review source repository, branch, head SHA, and the remote and destination ref that would receive the push. Prove that the selected remote has exactly one push URL identifying the review source repository, the destination ref is the review source branch, the current branch is the source branch, and the provider head SHA equals `HEAD` or is an ancestor with only coherent local commits ahead. If this cannot be proven, stop with the exact checkout blocker; this skill does not authorize switching branches.
4. Ignore stale, resolved, outdated, or other-branch signals unless they still describe the active blocker. If there is no target review, no blocker data, or no way to separate blocker fixes from unrelated local changes, stop and report it.

Pick the first blocker in this order unless the user named a narrower focus: conflicts or update-required; failed required or merge-blocking checks, including merge queue and synthetic-merge runs; actionable unresolved comments (people or bots) and required tasks; missing/skipped/canceled/pending required gates; human, policy, or tooling blockers. Once a class is absent for the current SHA, skip it until a push, rerun, target update, or completed wait. Recheck comments after every wait.

## Playbooks

### Conflict or update required

1. Use the exact remote target branch from the PR, else the resolved remote default. Never merge a local target branch.
2. Do not start a merge with unrelated local changes; finish the intended blocker-fix batch first or ask how to split.
3. Fetch the remote target. Take the update strategy from repo docs, branch policy, or sibling practice. If it requires rebase, linear history, or force-push, stop unless authorized; otherwise merge.
4. Resolve conflicts preserving branch intent and incoming changes; never pick a side wholesale. Read both sides' history (`git log --merge -p -- <file>`). Accept incoming removals of flags, dead code, or deprecated APIs this branch did not introduce, and adapt branch code.
5. Search for remaining markers, run the most relevant check on the merge result, fix merge-caused failures, require `git ls-files -u` to be empty, then commit.

### Failed check

1. Capture the failing source before editing: gate, URL/UUID, SHA, command, step, exit code, diagnostic, and key log lines. If logs are inaccessible, refresh auth once, retry once, then report `tooling-blocked`.
2. Timeouts, worker loss, outages, rate limits, or known nondeterminism without branch-caused evidence are infrastructure: one safe rerun or bounded wait, no code edits.
3. Reproduce with the closest local command when feasible; otherwise work from CI evidence and state the gap.
4. Fix the root cause. No suppressions, baselines, skipped/weakened tests, sleeps, broad catches, silent fallbacks, fake success exits, or retry wrappers that hide deterministic defects.
5. Prove the fix with the check tied to the original diagnostic; a broad command counts only if it demonstrably covers it. Do not rerun a required check already green for the latest SHA.

### Review comment or task

Apply the `address-comments` classification and fix rules, but publish through this loop. Fetch unresolved comments once per source SHA. Classify from evidence (`correct`, `unclear`, `subjective`, `wrong`, `YAGNI`, `already-addressed`); fix `correct` ones in the smallest coherent batch, ask one concise question for `unclear` ones, and leave the rest unless the user chooses otherwise. Do not resolve threads, reply, or dismiss reviews unless asked.

### Missing, skipped, canceled, or pending gate

Identify the gate, SHA, state, and whether it is required. Rerun a missing, skipped, or canceled gate once for that unchanged state when allowed; otherwise wait. Never rerun the same unchanged state twice. After each wait, refresh every surface that fed the active blocker and reconcile red against pending. Stop at `waiting` only for a status-only request, an explicit limit, a stuck gate, blocked provider access, or a state that needs human action.

### Human, policy, or tooling blocker

Report the exact blocker and the action required. Do not churn code to clear it.

## Publish and refresh

1. With no diff and no merge commit or provider-state change, do not commit or push. Stage only blocker-fix or target-update changes; ask how to split if unrelated work is mixed in.
2. Commit with a grounded Conventional Commit subject; the default merge message is fine for a target merge.
3. Push with `git push <selected-remote> HEAD:<review-source-branch>`, adding `-u` only when no upstream exists. If the pushed fixes change what the PR does, regenerate its title and description with `branch-description` and replace them on the PR; this is the only PR metadata write this skill makes.
4. After a push, update, rerun, or wait, refresh one bounded snapshot for the latest SHAs across every previously observed surface. Never collapse `waiting`, red non-required CI, or unknown data into "no local-fixable blockers" without naming the remaining state. Do not loop because the UI still shows a comment the pushed diff already addresses.

Each loop must follow a fix, target update, rerun, bounded wait, narrowed hypothesis, or new evidence; never blindly refetch, rerun, repush, or restart merge queues.

## Final

Finish only at `green`, `human-blocked`, `tooling-blocked`, or `waiting` under a stop condition above. Lead with CI state and merge-blocker state together. Report the target, latest checked SHAs, remaining gates or human actions, helper and provider evidence, fixes and proof, comments addressed or left with reasons, commit/push/rerun results, and evidence gaps. Keep it concise.
