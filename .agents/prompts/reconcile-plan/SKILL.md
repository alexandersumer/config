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

<intent>
Reconcile planning artifacts with code reality. Update planning docs only; never edit product code.
</intent>

<inputs>
- `artifact`: path/name/text for a spec, plan, design, proposal, roadmap, project, task artifact, or discovery focus.
- `scope`: optional path, behavior, project root, or branch/base comparison scope.
</inputs>

<resolve_artifacts>
Use `artifact`, else `$ARGUMENTS`, else discover repo-native planning artifacts broadly: `.plan/*`, `.projects/*`, `.tasks/*`, docs planning dirs, and common `SPEC/DESIGN/PROPOSAL/PLAN/ROADMAP/TODO/tasks` filenames.

Prefer artifact sets over single files: primary spec/design/proposal, implementation plan, project tracker, task list, roadmap/TODO, and linked companions. Score candidates by explicit input, `scope` locality, branch diff paths, recent modification, implementation-plan role, and references to touched code.

Emit `Artifact set:` before analysis. If no artifact set is found, top candidates are close, or the chosen set lacks locality evidence, ask one batched question and stop.
</resolve_artifacts>

<read_reality>
Read the full artifact set before judging status. Then read primary implementation evidence:
- Current branch, base/default branch, staged/unstaged changes, and `git diff base...HEAD` when available.
- Recent branch/default commits when relevant.
- Code, tests, config, docs, and entry points named by artifacts.
- Nearby tests and sibling implementations for wiring and convention checks.

Use `scope` to limit search, not to ignore artifact requirements. Emit `Reality sources:` before edits.
</read_reality>

<classify>
Classify every material requirement, chunk, acceptance criterion, decision, status marker, TODO, and deferred item:
- `complete`: reachable implementation plus test/check evidence.
- `partial`: some code exists, but acceptance, wiring, or verification is incomplete.
- `not-started`: no meaningful implementation evidence.
- `obsolete`: no longer applies because direction changed.
- `superseded`: replaced by another implemented design or repo convention.
- `blocked`: valid but waiting on an explicit dependency or decision.
- `unknown`: evidence is insufficient.

For each item, record artifact source, state, evidence, and doc action. Never mark complete from prose, TODOs, types/interfaces, generated code, or unreachable scaffolding alone.
</classify>

<update>
Edit planning artifacts only when the update is narrow and evidence-backed:
- Preserve repo-native front matter, headings, checklist/status format, and tone.
- Prefer factual edits: status, checkboxes, acceptance criteria, next chunk, deferred items, file references, and supersession notes.
- Keep useful historical decisions; annotate supersession instead of erasing context.
- Reconcile artifact conflicts against code reality, while preserving intent that still matters.
- Do not invent scope, milestones, owners, dates, metrics, or external references.
- Do not edit product code, tests, generated files, lockfiles, or unrelated docs.
- Do not delete pending/deferred items unless evidence shows `obsolete` or `superseded`.

If an update is broad, ambiguous, or product-significant, leave it unchanged and list it under Open questions.
</update>

<next_chunk>
Name the next valid artifact-backed chunk after reconciliation. It must be not complete, still relevant, wired through real entry points, reviewable, and have one observable acceptance signal. Reject docs/tests/types/helpers-only chunks unless paired with behavior. Do not branch or implement.
</next_chunk>

<verify>
Before final output:
- Re-read edited artifacts and confirm internal consistency.
- Confirm each edit maps to evidence or an Open questions item.
- Run lightweight validation such as `git diff --check` and file/front matter checks when practical.
- Confirm no product code changed.
</verify>

<output_format>
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
Keep under 35 lines. No preamble, summary, or sign-off.
</output_format>

<acceptance_criteria>
- Artifact set and reality sources emitted before edits.
- Every material artifact item is classified or listed as an open question.
- Updates are narrow, evidence-backed, and limited to planning artifacts.
- `complete` requires reachable implementation and verification evidence.
- Artifact set is internally consistent after edits.
- Next chunk names a valid chunk or explains why none qualifies.
- Final output matches the required shape.
</acceptance_criteria>
