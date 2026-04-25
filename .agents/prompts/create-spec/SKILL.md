---
name: create-spec
description: Author a tight technical spec plus a chunky implementation plan for execute-spec — no fluff, no filler
argument-hint: "[optional: feature/topic, problem statement, or path to seed notes]"
inputs:
  - name: topic
    label: Topic or seed
    description: One-line feature/topic, a problem statement, or a path to seed notes (e.g. `.plan/seed/foo.md`). Leave empty to infer from conversation context.
    type: string
    required: false
  - name: out_path
    label: Output path
    description: Optional override for where the spec is written. Leave empty to auto-pick (`.plan/specs/`, `.plan/changes/<slug>/design.md`, or `docs/specs/` based on what already exists in the repo). A sibling plan is always written too.
    type: string
    required: false
---

<intent>
Produce TWO self-contained planning artifacts that another engineer (or `execute-spec`) can implement without follow-up questions:

1. A tight, decision-rich technical spec: what to build, why, constraints, design, files, and observable acceptance.
2. A tight, chunky implementation plan: how to ship substantial end-to-end work slices, favoring meaningful chunks over tiny PRs.

Both artifacts must be ruthlessly scoped, concrete, and free of fluff.
</intent>

<inputs>
- `topic`: the seed for the spec and plan — a one-liner, a problem statement, or a path to existing notes. If empty, infer from the preceding conversation. If still ambiguous on scope or intent, ask ONE batched clarifying question and stop.
- `out_path`: optional override for where the spec file lands. The implementation plan must be written next to it using the same slug.
</inputs>

<learn_repo_conventions>
In one parallel batch of reads:
- `git ls-files '.plan/*' '.projects/*' 'docs/specs/*' 'docs/design/*' 'docs/rfcs/*' '*/SPEC.md' '*/DESIGN.md' '*/PROPOSAL.md' '*/PLAN.md' '*/ROADMAP.md'`.
- Read the 2 most recent existing planning artifacts. Mirror their front matter, heading order, and tone only where doing so preserves the requirements below.
- `README.md`, `CONTRIBUTING.md`, `AGENTS.md` / `CLAUDE.md` if present.
- Directories the topic touches, plus their nearest tests.

External reads (reference repos, prior art outside this workspace) are for learning only. Translate any pattern into this repo's vocabulary before it appears in either artifact. Never cite an external path, repo, URL, article, tool vendor, benchmark, or person in the output unless the topic is specifically about that external thing.

If no prior planning artifacts exist, use <spec_shape> and <plan_shape> and place both files under `.plan/specs/`.

Before writing, emit up to four convention lines: `Conventions: <path> for <aspect>`.
</learn_repo_conventions>

<resolve_location>
Pick the spec output path in this order:
1. `out_path` input if provided.
2. If existing specs live under `.plan/specs/`, write `.plan/specs/<NN>-<slug>.md` (next available number).
3. If existing specs live under `.plan/changes/<name>/`, write `.plan/changes/<slug>/design.md` and a sibling `proposal.md` (short "why + what changes" companion).
4. If under `docs/specs/` or `docs/rfcs/`, mirror that.
5. Otherwise create `.plan/specs/<slug>.md`.

Pick the plan output path as a sibling of the spec:
- If the spec is named `design.md`, write `plan.md` in the same directory.
- Otherwise write `<spec-stem>-plan.md` next to the spec. Example: `.plan/specs/004-search.md` gets `.plan/specs/004-search-plan.md`.

`<slug>` is kebab-case, derived from the topic, ≤6 words.

Before writing, emit two path lines:
- `Spec path: <path>`
- `Plan path: <path>`
</resolve_location>

<spec_shape>
The spec MUST contain these sections in this order. Omit a section ONLY if it is genuinely N/A and say so in one line.

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
- <explicit exclusion> — <one-line reason or "deferred to <next milestone/spec>">
- <explicit exclusion>

## Decisions

### Decision 1: <one-line claim, e.g. "Use Zod for boundary validation">

**Rationale:** 1–3 sentences citing constraints, existing patterns in the repo, or measured trade-offs.

**Rejected alternatives:**
- <alternative> — <one-line reason rejected>
- <alternative> — <one-line reason rejected>

### Decision 2: ...

(Aim for 4–10 decisions. Each one names a rejected alternative — if there are none, the "decision" is probably not a decision.)

## Design

Just enough to implement. Include:
- Module / package layout (one short tree if helpful).
- Public interfaces or schemas as language-appropriate snippets (the repo's primary language, plus YAML/JSON for config) — small, complete, copyable. NOT walls of code.
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

- <item> — <one-line reason or pointer to follow-up spec>
- <item>
```

If the repo's existing-spec convention differs (e.g. an `## Impact` or `## Capabilities` section), adopt theirs only if the resulting spec remains observable, decision-rich, and scoped.
</spec_shape>

<plan_shape>
The implementation plan MUST be a separate file. It MUST favor chunky work: substantial, end-to-end slices that produce real reachable behavior and are worth reviewing. Do not design a sequence of tiny PRs for isolated helpers, renames, pure types, plumbing-only work, or one-file nibbles unless the entire project is genuinely that small.

The plan MUST contain these sections in this order:

```
---
status: DRAFT
slug: <kebab-case-slug>
spec: <relative path to spec file>
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
- Every chunk MUST map to one or more spec acceptance criteria.
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
- The spec and plan MUST NOT name people, cite external repos, include irrelevant external references, or reference paths outside this repo. Use roles ("the implementer", "an operator").
- Use RFC-style "MUST / SHOULD / MAY" for implementer-facing invariants.
- Tables for option matrices; code fences only for interface snippets, tree diagrams, and shape examples.
- Concrete in-repo paths and type names — never "the appropriate module".
- Tight prose only: no motivational framing, no generic best-practice filler, no duplicated rationale, no "future possibilities" unless in Deferred / Follow-up with a reason.
- Length scales with scope. Single-change specs typically run 150–600 lines; plans should be shorter than the spec unless the rollout is unusually complex.
</style_exemplar>

<write>
If you cannot resolve scope or intent from the topic + repo reads, ask ONE batched clarifying question and stop.

Otherwise:
- Read per <learn_repo_conventions>.
- Write the spec file at the resolved spec path per <spec_shape> and <style_exemplar>.
- Write the plan file at the resolved plan path per <plan_shape> and <style_exemplar>.
- Write `proposal.md` companion if using the `changes/*` shape (1–3 paragraphs: problem, proposed change, files affected). This does not replace the plan.
- Emit two lines:
  - `Spec written: <path>`
  - `Plan written: <path>`
</write>

<acceptance_criteria>
- The spec file exists at the resolved path with front matter, all <spec_shape> sections, and <style_exemplar> invariants satisfied.
- The plan file exists at the resolved path with front matter, all <plan_shape> sections, and <style_exemplar> invariants satisfied.
- The plan references the spec by relative path.
- Every Decision names at least one rejected alternative.
- Every Acceptance Criterion is observable and falsifiable.
- Every plan chunk has a concrete acceptance signal and maps to one or more spec acceptance criteria.
- The plan favors chunky end-to-end slices: no standalone helper/type/test/docs-only chunks unless the whole project scope is that small and the plan says why.
- Files Changed table and plan Files lists reference real paths in this repo (or mark them `NEW`).
- Neither artifact contains person names, irrelevant external references, external repo references, URLs, or paths outside this repo.
- Output is the files themselves. Final status is exactly two lines: `Spec written: <path>` and `Plan written: <path>`. No workflow suggestions, no narration.
</acceptance_criteria>
