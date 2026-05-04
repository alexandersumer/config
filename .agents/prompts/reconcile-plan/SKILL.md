---
name: reconcile-plan
description: Reconcile planning artifacts with current codebase and branch state
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

Reconcile planning artifacts with code reality. Edit planning docs only. Never edit product code.

Do not satisfy this by making the plan sound current. The known failure mode is marking prose complete because other prose says so, deleting inconvenient future work, or updating docs without code evidence. A planning item is complete only when reachable implementation and verification evidence exist.

Resolve the artifact set:
- Use `artifact`, else `$ARGUMENTS`, else discover `.plan`, `.projects`, `.tasks`, docs planning dirs, and common spec/design/proposal/plan/roadmap/todo files.
- Prefer sets: primary artifact, implementation plan, tracker/task list, roadmap/TODO, linked companions.
- Score by explicit input, `scope` locality, branch diff paths, recency, plan role, and references to touched code.
- Emit `Artifact set:` before analysis. If no artifact scores, top candidates are close, or locality is weak, ask one batched question and stop.

Read reality before editing:
- Full artifact set.
- Current branch, base/default branch, staged/unstaged changes, and `git diff base...HEAD` when available.
- Code, tests, config, docs, and entry points named by artifacts.
- Nearby tests and sibling implementations.
- Emit `Reality sources:` before edits.

Classify every material requirement, chunk, acceptance criterion, decision, TODO, status marker, and deferred item:
- `complete`: reachable implementation plus test/check evidence.
- `partial`: some code exists but acceptance, wiring, or verification is incomplete.
- `not-started`: no meaningful implementation evidence.
- `obsolete`: no longer applies because direction changed.
- `superseded`: replaced by another implemented design or repo convention.
- `blocked`: valid but waiting on explicit dependency or decision.
- `unknown`: evidence is insufficient.

Update only narrow, evidence-backed planning facts: status, checkboxes, acceptance criteria, next chunk, deferred items, file references, supersession notes. Preserve useful decisions; annotate supersession instead of erasing context. Do not invent scope, milestones, owners, dates, metrics, or external references. Do not delete pending/deferred work unless evidence proves obsolete or superseded.

Name the next valid artifact-backed chunk after reconciliation. It must be incomplete, still relevant, reachable through real entry points, reviewable, and have one observable acceptance signal. Do not choose docs/tests/types/helpers-only work unless paired with behavior.

Verify before final output:
- Re-read edited artifacts for consistency.
- Confirm each edit maps to evidence or an open question.
- Run lightweight validation such as `git diff --check` when practical.
- Confirm no product code changed.

Final output under 35 lines:
```text
Artifact set:
- <path> — <role>

Reality sources:
- <source>

Reconciled:
- <artifact item>: <state> — <action taken or left unchanged> — evidence: <code/test/diff ref>

Updated files:
- <path or None>

Next chunk:
- <name or None>: <acceptance signal or reason>

Open questions:
- <item or None>

Checks: `<command>` -> <result>
```
