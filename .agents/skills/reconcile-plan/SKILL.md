---
name: reconcile-plan
description: Reconcile plan with code
---

Update structured planning state to match code reality. Do not edit product code.

Do not reconcile prose with prose. Complete means reachable implementation plus verification evidence.

Resolve the artifact set from `artifact`, `$ARGUMENTS`, or repo planning files. Use `scope` to narrow the code path, effective branch/worktree diff, behavior, or locality being reconciled. If locality is unclear, ask one question and stop.

Read the artifacts, effective branch/worktree diff and status, scoped code/tests/config/docs, nearby tests, and sibling implementations. The effective diff includes committed branch changes, `git diff --cached`, `git diff`, and untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs. Treat committed and uncommitted code as reality. Treat a check as evidence only when it is visible, same-scope, after the last relevant edit, and not invalidated by later changes or environment state. Treat planning text as a claim to verify.

Classify only what you can prove: `complete`, `partial`, `not-started`, `obsolete`, `superseded`, `blocked`, or `unknown`. Unknown is better than fake certainty.

Preserve the artifact's structure. Update existing planning state in place: statuses, checkboxes, acceptance, next work, deferred/obsolete items, and stale refs. Do not add branch-evidence prose or reconciliation notes that do not belong to an existing task. Do not invent owners, dates, milestones, metrics, scope, or completion. Do not delete pending work unless code proves it obsolete or superseded.

Name the next incomplete, relevant, reachable, reviewable chunk with one acceptance signal.

Final response should be short: what changed, what evidence justified it, what remains next, and what could not be proven. No reconciliation theater.
