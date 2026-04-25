---
name: create-spec
description: Author a tight planning artifact plus a chunky implementation plan — no fluff, no filler
argument-hint: "[optional: feature/topic, problem statement, or path to seed notes/artifact]"
inputs:
  - name: topic
    label: Topic or seed
    description: One-line feature/topic, a problem statement, or a path to seed notes/artifact (e.g. `.plan/seed/foo.md`, `docs/design/foo.md`). Leave empty to infer from conversation context.
    type: string
    required: false
  - name: out_path
    label: Output path
    description: Optional override for where the primary planning artifact is written. Leave empty to auto-pick from existing repo artifact conventions. A companion implementation plan is always written too.
    type: string
    required: false
---

<intent>
Produce TWO self-contained planning artifacts that another engineer (or `execute-spec`) can implement without follow-up questions:

1. A tight, decision-rich primary artifact: what to build, why, constraints, design, files, and observable acceptance. This may be called a spec, design, proposal, RFC, change document, or another repo-native name.
2. A tight, chunky implementation plan: how to ship substantial end-to-end work slices, favoring meaningful chunks over tiny PRs.

Both artifacts must be ruthlessly scoped, concrete, and free of fluff.
</intent>

<inputs>
- `topic`: the seed for the planning artifact and plan — a one-liner, a problem statement, or a path to existing notes/artifacts. If empty, infer from the preceding conversation. If still ambiguous on scope or intent, ask ONE batched clarifying question and stop.
- `out_path`: optional override for where the primary planning artifact lands. The implementation plan should be written next to it using the same slug unless repo conventions clearly separate design/spec/proposal artifacts from plans.
</inputs>

<learn_repo_conventions>
In one parallel batch of reads:
- `git ls-files '.plan/*' '.projects/*' 'docs/specs/*' 'docs/design/*' 'docs/plans/*' 'docs/rfcs/*' '*/SPEC.md' '*/spec.md' '*/DESIGN.md' '*/design.md' '*/PROPOSAL.md' '*/proposal.md' '*/PLAN.md' '*/plan.md' '*/ROADMAP.md' '*/roadmap.md' '*/TODO.md' '*/todo.md' '*/tasks.md'`.
- Read up to 4 existing planning artifacts, prioritizing the most relevant by path, name, recency, and topic overlap. Mirror their front matter, heading order, and tone only where doing so preserves the requirements below.
- `README.md`, `CONTRIBUTING.md`, and repo-local agent instruction files such as `AGENTS.md` if present.
- Directories the topic touches, plus their nearest tests.

External reads (reference repos, prior art outside this workspace) are for learning only. Translate any pattern into this repo's vocabulary before it appears in either artifact. Never cite an external path, repo, URL, article, tool vendor, benchmark, or person in the output unless the topic is specifically about that external thing.

If no prior planning artifacts exist, use <primary_artifact_shape> and <plan_shape> and place both files under `.plan/` using clear names (`<slug>.md` and `<slug>-plan.md`).

Before writing, emit up to four convention lines: `Conventions: <path> for <aspect>`.
</learn_repo_conventions>

<resolve_location>
Pick the primary artifact output path in this order:
1. `out_path` input if provided.
2. If existing artifacts for the same kind of work use a clear directory and filename convention, mirror that convention. Preserve repo-native names such as `design.md`, `proposal.md`, `spec.md`, `PLAN.md`, `ROADMAP.md`, numbered files, or change directories when they are the local convention.
3. If the repo uses change directories such as `.plan/changes/<name>/`, write the primary artifact in a new change directory using the repo's established primary filename. Add companion artifacts only if companions already appear in that convention.
4. If artifacts live under `.plan/specs/`, `docs/specs/`, `docs/design/`, `docs/plans/`, or `docs/rfcs/`, mirror that directory and numbering style.
5. Otherwise create `.plan/<slug>.md`.

Pick the implementation plan output path by convention:
- If the repo already separates plans from specs/designs/proposals, mirror that separation.
- If the primary artifact is inside a change directory, write `plan.md` in the same directory.
- Otherwise write `<primary-artifact-stem>-plan.md` next to the primary artifact.

`<slug>` is kebab-case, derived from the topic, ≤6 words.

Before writing, emit two path lines:
- `Artifact path: <path>`
- `Plan path: <path>`
</resolve_location>

<primary_artifact_shape>
The primary planning artifact MUST contain these sections in this order unless the repo has an existing equivalent structure. Omit a section ONLY if it is genuinely N/A and say so in one line.

```
---
status: DRAFT
slug: <kebab-case-slug>
date: <YYYY-MM-DD>
---

# <Title>

## Problem

What exists today, what it cannot do, and why that matters now. Concrete, not "we want better X". 1–3 short paragraphs. Cite specific files / modules / behaviours where helpful.

## Goals / Non-Goals

**Goals:**
- <observable outcome>
- <observable outcome>

**Non-Goals:**
- <explicit exclusion> — <one-line reason or "deferred to <next milestone/artifact>">
- <explicit exclusion>

## Decisions

### Decision 1: <one-line claim, e.g. "Validate requests at the boundary">

**Rationale:** 1–3 sentences citing constraints, existing patterns in the repo, or measured trade-offs.

**Rejected alternatives:**
- <alternative> — <one-line reason rejected>
- <alternative> — <one-line reason rejected>

### Decision 2: ...

(Aim for 4–10 decisions. Each one names a rejected alternative — if there are none, the "decision" is probably not a decision.)

## Design

Just enough to implement. Include:
- Module / package layout (one short tree if helpful).
- Public interfaces, schemas, config, or contracts as language-appropriate snippets — small, complete, copyable. NOT walls of code.
- Cross-module flow only when not obvious from the interfaces.

Use sub-headings per component when it helps; otherwise one section is fine.

## Files Changed

| File | Change |
|------|--------|
| `<path>` | <one-line summary> |
| `<path>` | <one-line summary> |

## Acceptance Criteria

Given / When / Then, observable. Each criterion must be checkable by a test, a CLI run, an HTTP call, or a log line.

**Given** <precondition>
**When** <action>
**Then** <observable result>

(5–10 criteria. Cover happy path, at least one failure path, at least one edge case.)

## Out of Scope

- <item> — <one-line reason or pointer to follow-up artifact>
- <item>
```

If the repo's existing planning-artifact convention differs (e.g. an `## Impact`, `## Capabilities`, or `## Proposal` section), adopt theirs only if the resulting artifact remains observable, decision-rich, and scoped.
</primary_artifact_shape>

<plan_shape>
The implementation plan MUST be a separate file. It MUST favor chunky work: substantial, end-to-end slices that produce real reachable behavior and are worth reviewing. Do not design a sequence of tiny PRs for isolated helpers, renames, pure types, plumbing-only work, or one-file nibbles unless the entire project is genuinely that small.

The plan MUST contain these sections in this order:

```
---
status: DRAFT
slug: <kebab-case-slug>
artifact: <relative path to primary planning artifact>
date: <YYYY-MM-DD>
---

# <Title> Implementation Plan

## Strategy

1–3 short paragraphs describing the delivery order and why it minimizes risk. Name the main dependency chain. State what must be shipped together to avoid dead code or tiny PRs.

## Chunking Rules

- Prefer chunks that deliver one complete user-visible behavior or one complete internal capability wired into real entry points.
- Each chunk SHOULD usually touch multiple related files when that is necessary to be end-to-end; do not split tests, wiring, docs, and production code into separate chunks.
- Avoid tiny chunks such as "add type", "rename field", "add helper", "write docs", or "add tests" unless paired with the production behavior they support.
- Avoid huge rewrites; split only at stable seams where each chunk can be verified independently.
- Each chunk MUST have an observable acceptance signal.

## Chunks

### Chunk 1: <verb + noun, substantial slice>

**Goal:** <one complete behavior/capability this chunk ships>

**Scope:**
- <production code + wiring>
- <tests>
- <docs/config/migration if required>

**Files:**
- `<path>` — <change>
- `<path>` — <change>

**Acceptance signal:** `<command/test/CLI/HTTP/log check>` demonstrates <observable result>.

**Depends on:** None.

### Chunk 2: ...

(Use the fewest chunks that still keep reviewable, independently verifiable work. Most medium features should be 2–5 chunks, not 10–20.)

## Verification

- <full-suite command or repo-standard check>
- <targeted test command(s) mapped to acceptance criteria>
- <manual/CLI/API check if needed>

## Deferred / Follow-up

- <item> — <reason, or "None">
```

Chunk quality bar:
- A chunk is too small if it only moves code around, adds scaffolding, updates types, or writes tests without shipping reachable behavior.
- A chunk is too large if it cannot be reviewed coherently or lacks a single dominant acceptance signal.
- Every chunk MUST map to one or more primary-artifact acceptance criteria or explicitly stated outcomes.
</plan_shape>

<style_exemplar>
Match the shape, tone, and density of these examples. Substitute names from this repo.

Decision (language-agnostic example; substitute concepts native to the repo's stack):

```
### Decision 2: Validate at the boundary, not at every call site

**Rationale:** Validation logic duplicated across handlers drifts. Centralising at the request boundary matches how `<repo-existing-module>` already gates inputs and lets handlers assume parsed types. Cost is one extra allocation per request, measured small.

**Rejected alternatives:**
- Per-handler validation — proven to drift in `<repo-existing-module>`; divergent copies removed previously.
- Schema-as-type-only (no runtime check) — silently accepts malformed inputs from non-typed callers.
```

Chunk:

```
### Chunk 2: Route report generation through the new renderer

**Goal:** The CLI uses the renderer for text and JSON output, with both modes covered by golden tests.

**Scope:**
- Wire the renderer into `<repo-cli-entrypoint>`.
- Add golden tests for text and JSON modes.
- Update user-facing help text for the new mode.

**Acceptance signal:** `<test command>` passes and fails if the renderer call is bypassed.
```

Style invariants:
- The primary artifact and plan MUST NOT name people, cite external repos, include irrelevant external references, or reference paths outside this repo. Use roles ("the implementer", "an operator").
- Use RFC-style "MUST / SHOULD / MAY" for implementer-facing invariants.
- Tables for option matrices; code fences only for interface snippets, tree diagrams, and shape examples.
- Concrete in-repo paths and type names — never "the appropriate module".
- Tight prose only: no motivational framing, no generic best-practice filler, no duplicated rationale, no "future possibilities" unless in Deferred / Follow-up with a reason.
- Length scales with scope. Single-change planning artifacts typically run 150–600 lines; plans should be shorter than the primary artifact unless the rollout is unusually complex.
</style_exemplar>

<write>
If you cannot resolve scope or intent from the topic + repo reads, ask ONE batched clarifying question and stop.

Otherwise:
- Read per <learn_repo_conventions>.
- Write the primary planning artifact at the resolved artifact path per <primary_artifact_shape> and <style_exemplar>.
- Write the implementation plan at the resolved plan path per <plan_shape> and <style_exemplar>.
- Write companion artifacts such as `proposal.md` only when that companion is part of the repo's existing convention. This does not replace the plan.
- Emit two lines:
  - `Artifact written: <path>`
  - `Plan written: <path>`
</write>

<acceptance_criteria>
- The primary planning artifact exists at the resolved path with front matter, <primary_artifact_shape> content or a repo-native equivalent, and <style_exemplar> invariants satisfied.
- The implementation plan exists at the resolved path with front matter, <plan_shape> content or a repo-native equivalent, and <style_exemplar> invariants satisfied.
- The plan references the primary artifact by relative path.
- Every Decision names at least one rejected alternative unless the repo-native artifact structure has an equivalent decision/option rationale format.
- Every Acceptance Criterion or outcome is observable and falsifiable.
- Every plan chunk has a concrete acceptance signal and maps to one or more primary-artifact acceptance criteria or outcomes.
- The plan favors chunky end-to-end slices: no standalone helper/type/test/docs-only chunks unless the whole project scope is that small and the plan says why.
- Files Changed table and plan Files lists reference real paths in this repo (or mark them `NEW`).
- Neither artifact contains person names, irrelevant external references, external repo references, URLs, or paths outside this repo.
- Output is the files themselves. Final status is exactly two lines: `Artifact written: <path>` and `Plan written: <path>`. No workflow suggestions, no narration.
</acceptance_criteria>
