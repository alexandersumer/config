---
name: create-spec
description: Author a tight planning artifact plus a chunky implementation plan — no fluff, no filler
argument-hint: "[optional: feature/topic, problem statement, or path to seed notes/artifact]"
inputs:
  - name: topic
    label: Topic or seed
    description: One-line feature/topic, problem statement, or path to seed notes/artifact. Leave empty to infer from conversation context.
    type: string
    required: false
  - name: out_path
    label: Output path
    description: Optional path for the primary planning artifact. Leave empty to mirror repo artifact conventions.
    type: string
    required: false
---

<intent>
Create two implementation-ready planning artifacts:
1. A primary artifact (spec/design/proposal/RFC/change doc/etc.) that states what to build, why, constraints, decisions, affected files, and observable acceptance.
2. A companion implementation plan with chunky, reviewable, end-to-end slices.

Keep both scoped, concrete, repo-native, and free of filler.
</intent>

<inputs>
- `topic`: one-liner, problem statement, path to notes/artifact, or empty to infer from context. If scope/intent remains ambiguous, ask one batched question and stop.
- `out_path`: optional path for the primary artifact. Write the plan next to it unless repo conventions clearly separate artifact types.
</inputs>

<learn_conventions>
In one parallel batch, read:
- Planning artifacts: `.plan/*`, `.projects/*`, `.tasks/*`, `docs/specs/*`, `docs/design/*`, `docs/plans/*`, `docs/rfcs/*`, and common `SPEC/DESIGN/PROPOSAL/PLAN/ROADMAP/TODO/tasks` filenames, case-insensitive where practical.
- Up to 4 relevant existing artifacts, prioritizing topic/path/recency overlap.
- `README.md`, `CONTRIBUTING.md`, repo-local agent instructions such as `AGENTS.md`, touched directories, and nearest tests.

Mirror existing artifact location, front matter, headings, and tone only when that preserves the requirements below. External reads are for learning only; do not cite external repos, URLs, people, or paths unless the topic is specifically external.

If no convention exists, write `.plan/<slug>.md` and `.plan/<slug>-plan.md`.
Emit before writing: `Conventions: <path> for <aspect>` lines, max 4.
</learn_conventions>

<resolve_paths>
Pick paths before writing:
- Primary artifact: `out_path`, else same-kind repo convention, else change-directory convention, else existing planning/docs convention, else `.plan/<slug>.md`.
- Plan: repo plan convention if separate, else `plan.md` in a change directory, else `<primary-stem>-plan.md` next to the primary artifact.
- `<slug>`: kebab-case, derived from topic, ≤6 words.

Emit:
- `Artifact path: <path>`
- `Plan path: <path>`
</resolve_paths>

<primary_artifact_contract>
Use repo-native structure if it stays observable, decision-rich, and scoped. Otherwise include:
- Front matter: `status: DRAFT`, `slug`, `date`.
- Problem: current state, gap, why now, concrete repo references.
- Goals / Non-Goals: observable outcomes and explicit exclusions.
- Decisions: only material decisions, each with rationale and rejected alternatives when there was a real trade-off. Do not invent decisions to hit a count.
- Design: implementable module/package/interface/config details; small snippets only.
- Files Changed: real repo paths or `NEW`, each with one-line change.
- Acceptance Criteria: observable, falsifiable checks covering the important happy paths and realistic failure/edge cases for this scope.
- Out of Scope: deferred items with reasons or follow-up artifact pointers.
</primary_artifact_contract>

<plan_contract>
The plan is a separate artifact with front matter referencing the primary artifact. It must include:
- Strategy: delivery order, dependency chain, what must ship together.
- Chunking Rules: prefer end-to-end slices; avoid helper/type/test/docs-only nibbles unless paired with behavior; avoid huge rewrites; each chunk has an observable acceptance signal.
- Chunks: 2–5 for most medium work. Each chunk has Goal, Scope, Files, Acceptance signal, Depends on.
- Verification: full-suite and targeted checks.
- Deferred / Follow-up: item plus reason, or `None`.

Every chunk maps to primary-artifact criteria/outcomes. Too small: scaffolding/types/tests/docs without reachable behavior. Too large: incoherent review or no dominant acceptance signal.
</plan_contract>

<style>
- Use repo vocabulary and concrete in-repo paths/types; never “appropriate module”.
- Use MUST/SHOULD/MAY for implementer-facing invariants.
- Tables for matrices; code fences only for snippets/trees/examples.
- No people names, irrelevant external refs, external repo refs, URLs, or paths outside this repo.
- Tight prose. No motivational framing, duplicated rationale, or speculative future work outside Deferred/Follow-up.
- Length scales with scope; plan should usually be shorter than the primary artifact.
</style>

<write>
If scope or intent is unresolved after repo reads, ask one batched question and stop. Otherwise write both artifacts, plus companion files only when repo convention requires them.

Final response exactly:
Artifact written: <path>
Plan written: <path>
</write>

<acceptance_criteria>
- Primary artifact and plan exist at resolved paths, with repo-native structure or the contracts above.
- Plan references the primary artifact by relative path.
- Decisions include rationale; rejected alternatives appear only when there was a meaningful trade-off.
- Criteria/outcomes are observable and falsifiable.
- Chunks are chunky, end-to-end, mapped to outcomes, and have concrete acceptance signals.
- Referenced paths are real or marked `NEW`.
- No irrelevant external/person references.
- Final response is exactly the two `... written:` lines.
</acceptance_criteria>
