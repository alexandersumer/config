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

Do not mark prose complete from other prose. Complete means reachable implementation plus verification evidence.

Resolve artifact set from `artifact`, `$ARGUMENTS`, or repo planning files. Emit `Artifact set:`. If locality is unclear, ask one question and stop.

Read artifacts, branch diff/status, named code/tests/config/docs, nearby tests, and sibling implementations. Emit `Reality sources:`.

Classify material items: `complete`, `partial`, `not-started`, `obsolete`, `superseded`, `blocked`, `unknown`.

Edit only narrow evidence-backed planning facts: status, checkboxes, acceptance, next chunk, deferred items, file refs, supersession notes. Do not invent owners, dates, milestones, metrics, or scope. Do not delete pending work unless obsolete or superseded.

Name the next incomplete, relevant, reachable, reviewable chunk with one acceptance signal.

Final:
```text
Artifact set:
- <path> — <role>
Reality sources:
- <source>
Reconciled:
- <item>: <state> — <action> — evidence: <ref>
Updated files:
- <path or None>
Next chunk:
- <name or None>: <signal or reason>
Open questions:
- <item or None>
Checks: `<command>` -> <result>
```
