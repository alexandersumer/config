---
name: reconcile-plan
description: Reconcile plan with code
---

Update structured planning state to match code reality. Do not edit product code.

Do not reconcile prose with prose. Complete means reachable implementation plus verification evidence.

Resolve the artifact set from `artifact`, `$ARGUMENTS`, or repo planning files. Use `scope` to narrow the code path, branch diff, behavior, or locality being reconciled. If locality is unclear, ask one question and stop.

Read the artifacts, branch diff/status, scoped code/tests/config/docs, nearby tests, and sibling implementations. Treat code and executed checks as reality. Treat planning text as a claim to verify.

Classify only what you can prove: `complete`, `partial`, `not-started`, `obsolete`, `superseded`, `blocked`, or `unknown`. Unknown is better than fake certainty.

Preserve the artifact's structure. Update existing planning state in place: statuses, checkboxes, acceptance, next work, deferred/obsolete items, and stale refs. Do not add branch-evidence prose or reconciliation notes that do not belong to an existing task. Do not invent owners, dates, milestones, metrics, scope, or completion. Do not delete pending work unless code proves it obsolete or superseded.

Name the next incomplete, relevant, reachable, reviewable chunk with one acceptance signal.

Final response should be short: what changed, what evidence justified it, what remains next, and what could not be proven. No reconciliation theater.
