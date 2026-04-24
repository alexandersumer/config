---
name: create-spec
description: Author a tight, decision-rich technical spec ready to hand to execute-spec — no fluff, no filler
argument-hint: "[optional: feature/topic, problem statement, or path to seed notes]"
inputs:
  - name: topic
    label: Topic or seed
    description: One-line feature/topic, a problem statement, or a path to seed notes (e.g. `.plan/seed/foo.md`). Leave empty to infer from conversation context.
    type: string
    required: false
  - name: out_path
    label: Output path
    description: Optional override for where the spec is written. Leave empty to auto-pick (`.plan/specs/`, `.plan/changes/<slug>/design.md`, or `docs/specs/` based on what already exists in the repo).
    type: string
    required: false
---

<intent>
Produce ONE self-contained technical spec that another engineer (or `execute-spec`) can implement without follow-up questions. Tight, decision-rich, observable, ruthlessly scoped.
</intent>

<inputs>
- `topic`: the seed for the spec — a one-liner, a problem statement, or a path to existing notes. If empty, infer from the preceding conversation. If still ambiguous on scope or intent, ask ONE batched clarifying question and stop.
- `out_path`: optional override for where the file lands.
</inputs>

<learn_repo_conventions>
In one parallel batch of reads:
- `git ls-files '.plan/*' '.projects/*' 'docs/specs/*' 'docs/design/*' 'docs/rfcs/*' '*/SPEC.md' '*/DESIGN.md' '*/PROPOSAL.md'`.
- Read the 2 most recent existing specs. Mirror their front matter, heading order, and tone.
- `README.md`, `CONTRIBUTING.md`, `AGENTS.md` / `CLAUDE.md` if present.
- Directories the topic touches, plus their nearest tests.

External reads (e.g. reference repos under `~/...`) are for your learning only. Translate any pattern into this repo's vocabulary before it appears in the spec. Never cite an external path, repo, or person in the spec output.

If no prior specs exist, use <spec_shape> and place the file under `.plan/specs/`.

Emit one line before writing: `Conventions: <path> for <aspect>` per pattern adopted. ≤4 lines.
</learn_repo_conventions>

<resolve_location>
Pick the output path in this order:
1. `out_path` input if provided.
2. If existing specs live under `.plan/specs/`, write `.plan/specs/<NN>-<slug>.md` (next available number).
3. If existing specs live under `.plan/changes/<name>/`, write `.plan/changes/<slug>/design.md` and a sibling `proposal.md` (short "why + what changes" companion).
4. If under `docs/specs/` or `docs/rfcs/`, mirror that.
5. Otherwise create `.plan/specs/<slug>.md`.

`<slug>` is kebab-case, derived from the topic, ≤6 words.

Output one line: `Spec path: <path>`.
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
- Public interfaces or schemas as TypeScript / Go / Python / YAML / JSON snippets — small, complete, copyable. NOT walls of code.
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

If the repo's existing-spec convention differs (e.g. an `## Impact` or `## Capabilities` section), adopt theirs. The intent — observable, decision-rich, scoped — does not change.
</spec_shape>

<style_exemplar>
Match the shape, tone, and density of these examples. Substitute names from this repo.

Decision:

```
### Decision 2: Validate at the boundary, not at every call site

**Rationale:** Validation logic duplicated across handlers drifts. Centralising at the request boundary matches how `<repo-existing-module>` already gates inputs and lets handlers assume parsed types. Cost is one extra allocation per request, measured at ~10µs.

**Rejected alternatives:**
- Per-handler validation — proven to drift in `<repo-existing-module>`; three divergent copies removed last quarter.
- Schema-as-type-only (no runtime check) — silently accepts malformed inputs from non-typed callers.
```

Acceptance criterion:

```
**Given** a request whose `<field>` exceeds the configured maximum
**When** the boundary validator runs
**Then** the response is HTTP 400 with body `{ "error": "<field>_too_large" }` and no handler code executes (verified by absence of the handler's log line)
```

Goals / Non-Goals are symmetric one-line bullets. Each Non-Goal carries a one-line reason or a pointer to a follow-up spec.

Style invariants:
- The spec MUST NOT name people, cite external repos, or reference paths outside this repo. Use roles ("the implementer", "an operator").
- Use RFC-style "MUST / SHOULD / MAY" for implementer-facing invariants.
- Tables for option matrices; code fences only for interface snippets and tree diagrams.
- Concrete in-repo paths and type names — never "the appropriate module".
- Length scales with scope. Single-change specs typically run 150–600 lines.
</style_exemplar>

<acceptance_criteria>
- A single line `Conventions: ...` was emitted before any file was written, citing concrete repo paths (or "no prior specs found" if none).
- A single line `Spec path: <path>` was emitted before any file was written.
- The spec file exists at the resolved path with the front matter, all sections, and the style rules satisfied.
- Every Decision in the spec names at least one rejected alternative.
- Every Acceptance Criterion in the spec is observable and falsifiable.
- The Files Changed table references real paths in this repo (or marks them `NEW` for files to be created).
- The spec contains no person names, no external repo references, and no paths outside this repo.
- No follow-up `.md` files were created beyond the spec itself (and `proposal.md` companion when using the `changes/*` shape).
</acceptance_criteria>

<output_format>
Final message uses exactly this shape, ≤20 lines, no preamble, no sign-off:

```
Conventions: <repo>:<path> for <aspect>
Spec path: <path>

Title: <title>
Decisions: <N> (<short list of one-word topics>)
Acceptance criteria: <N>
Out of scope: <N>

Open questions (if any):
- <one line>

Next: review with `grill-me`, then implement with `execute-spec`.
```
</output_format>
