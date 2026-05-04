---
name: reconcile-plan
description: Reconcile plan with code
argument-hint: "[optional: artifact path/name or scope]"
inputs:
  - name: artifact
    label: Planning artifact
    description: Optional spec, plan, project, task path/name/text, or discovery focus. Leave empty to discover relevant artifacts.
    type: string
    required: false
  - name: scope
    label: Reconciliation scope
    description: Optional code path, branch diff, project root, behavior, or branch/default-branch comparison scope. Leave empty to infer from branch and artifact locality.
    type: string
    required: false
---

Update planning artifacts to match code reality. Do not edit product code.

Do not reconcile prose with prose. Complete means reachable implementation plus verification evidence.

Resolve the artifact set from `artifact`, `$ARGUMENTS`, or repo planning files. Use `scope` to narrow the code path, branch diff, behavior, or locality being reconciled. If locality is unclear, ask one question and stop.

Read the artifacts, branch diff/status, scoped code/tests/config/docs, nearby tests, and sibling implementations. Treat code and executed checks as reality. Treat planning text as a claim to verify.

Classify only what you can prove: `complete`, `partial`, `not-started`, `obsolete`, `superseded`, `blocked`, or `unknown`. Unknown is better than fake certainty.

Edit only narrow evidence-backed planning facts: status, checkboxes, acceptance, next chunk, deferred items, file refs, supersession notes. Do not invent owners, dates, milestones, metrics, scope, or completion. Do not delete pending work unless code proves it obsolete or superseded.

Name the next incomplete, relevant, reachable, reviewable chunk with one acceptance signal.

Final response should be short: what changed, what evidence justified it, what remains next, and what could not be proven. No reconciliation theater.
