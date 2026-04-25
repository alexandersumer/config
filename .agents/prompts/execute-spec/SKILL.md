---
name: execute-spec
description: Execute a planning artifact end-to-end as substantial production-grade work, with canonical patterns and robust verification
argument-hint: "[optional: artifact path, plan name, or inline instructions]"
inputs:
  - name: artifact
    label: Planning artifact
    description: Path/name/text for a spec, plan, design, proposal, roadmap, task artifact, or inline instructions. Leave empty to discover the most relevant recent artifact.
    type: string
    required: false
  - name: spec
    label: Planning artifact (legacy alias)
    description: Backward-compatible alias for artifact. Prefer artifact for new usage.
    type: string
    required: false
---

<intent>
Deliver the artifact as substantial working software: production code, real wiring, meaningful tests, full verification. Not a sketch, docs-only pass, stub, or nibble.
</intent>

<resolve_artifact>
Use `artifact`, else legacy `spec`, else `$ARGUMENTS`.

If empty, discover repo-native planning artifacts broadly: `.plan/*`, `.projects/*`, `.tasks/*`, docs planning dirs, and common `SPEC/DESIGN/PROPOSAL/PLAN/ROADMAP/TODO/tasks` filenames. Prefer a relevant implementation plan; otherwise pick the most relevant recent artifact by path, recency, and content. Emit `Artifact: <path>`. If none exist, ask for an artifact or inline instructions and stop.

If input is a file, read it. If directory, read planning artifacts inside it, preferring plans before companion designs/proposals. If bare name, resolve across planning directories and common filenames. Otherwise treat as inline instructions.

Read the artifact set end to end before coding. Read linked companions too. If both primary artifact and plan exist, use the primary artifact for intent/scope and the plan for execution order. If scope, intent, order, or acceptance is ambiguous, ask one batched question and stop.
</resolve_artifact>

<scope_floor>
Ship the artifact’s intended scope. If too large, ship the next coherent vertical slice and name deferred work. Disqualified as substantial: docs/comments-only, types/interfaces-only, TODOs-only, one trivial helper, renames, reformatting. Use judgment; do not pad.
</scope_floor>

<learn_patterns>
Before writing, read in parallel:
- README/CONTRIBUTING/repo-local agent instructions.
- Directories the artifact touches and nearest tests.
- 2–3 sibling features in this repo with similar shape.
- 1–2 prior-art areas inside this repo when useful; do not inspect external repositories unless the artifact explicitly names them.

Emit `Canonical patterns: <path> for <aspect>` lines before editing.
</learn_patterns>

<plan>
Use update_todo. Preserve artifact order when it is already a plan unless dependencies require adjustment. Each task must be one observable behavior/capability with a checkable acceptance signal. Re-read the artifact set after planning; every requirement is in the task list or `Deferred:` with reason.
</plan>

<implement>
For each task:
- Build production code, wiring, docs/config, and tests together; unreachable code does not count.
- Mirror canonical patterns. Reuse existing helpers/types/fixtures. Add dependencies/layers only when required by the artifact.
- Tests cover the important happy paths and realistic failure/edge cases for this scope; each new test should catch a named plausible mutation or regression.
- Run the smallest relevant checks after each task. Run the full local suite before the final report only when available and reasonable for the repo; otherwise report the strongest targeted checks run. Fix application/test code. Disallowed: suppressions, baselines, dependency bumps, build-config edits, test-infra edits, skipped tests, shipped TODO placeholders.

Parallelize independent reads/edits. Do not narrate intermediate steps.
</implement>

<verify>
Before final report:
- Strongest relevant local checks are green; prefer the full suite when available and reasonable, otherwise state why targeted checks were used.
- Every acceptance signal demonstrated by an actual run, or explicitly listed as not runnable with the reason.
- Artifact set re-read; every requirement shipped or deferred with reason.
- Diff stays inside artifact scope; incidental refactors only when needed.
- Diff clears <scope_floor>.
</verify>

<acceptance_criteria>
- Artifact and canonical-pattern lines emitted before edits.
- Every artifact requirement shipped or deferred with reason.
- New code reachable from real entry points.
- Tests cover happy/failure/edge and named mutation.
- Final checks pass, or blocked checks are named with the blocking reason; no disallowed suppressions.
- Working tree contains changes, uncommitted, on current branch.
</acceptance_criteria>

<output_format>
```
Artifact: <path or "inline">
Canonical patterns:
- <repo>:<path> for <aspect>

Shipped:
- <behavior>: <files> — verified by `<command>` -> <result>

Diff size: <N files, ~M lines net>
Checks: `<command>` -> <pass summary>

Deferred (if any):
- <artifact item>: <one-line reason>

Next: review-branch, then create-pull-request.
```
Keep under 35 lines. No preamble, no sign-off.
</output_format>
