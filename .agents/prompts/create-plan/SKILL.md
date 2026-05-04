---
name: create-plan
description: Author a concise planning artifact plus a reviewable implementation plan
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

Create two implementation-ready artifacts: a primary spec/design/proposal and a companion implementation plan.

Do not satisfy this by writing generic planning prose. The known failure mode is a document that sounds reasonable but leaves the implementer to rediscover scope, decisions, files, tests, and acceptance. The deliverable is a repo-native artifact set that can drive coding without another clarification round.

Inputs:
- `topic`: one-liner, problem statement, path to notes/artifact, or empty to infer from context.
- `out_path`: optional primary artifact path. Put the plan next to it unless repo convention says otherwise.

Before writing, learn the repo convention in one focused pass:
- Read likely planning locations: `.plan`, `.projects`, `.tasks`, `docs/specs`, `docs/design`, `docs/plans`, `docs/rfcs`, and common spec/design/proposal/plan/todo files.
- Read up to four relevant existing artifacts, plus README/CONTRIBUTING/repo-local agent instructions and touched directories when known.
- Emit at most four lines: `Conventions: <path> for <aspect>`.

Resolve paths before writing:
- Primary: `out_path`, else matching repo convention, else `.plan/<slug>.md`.
- Plan: repo plan convention, else `<primary-stem>-plan.md` next to the primary.
- `<slug>` is kebab-case, derived from the topic, at most six words.
- Emit `Artifact path: <path>` and `Plan path: <path>`.

Primary artifact must include, in repo-native form or these headings:
- Problem: current state, gap, why now, concrete repo references.
- Goals and Non-Goals: observable outcomes and explicit exclusions.
- Decisions: material choices with rationale; rejected alternatives only when there was a real trade-off.
- Design: implementable module/package/interface/config details; snippets only when useful.
- Files Changed: real repo paths or `NEW`, each with a one-line change.
- Acceptance Criteria: falsifiable checks for happy paths and realistic edge/failure cases.
- Out of Scope: deferred items with reasons.

Plan must include:
- Strategy: delivery order and dependencies.
- Chunking rules: reviewable end-to-end slices; no docs/tests/types/helpers-only chunks unless paired with behavior.
- Chunks: usually 2-5, each with Goal, Scope, Files, Acceptance signal, Depends on.
- Verification: targeted and full checks.
- Deferred / Follow-up: item plus reason, or `None`.

Style:
- Use repo vocabulary and concrete paths/types.
- Use MUST/SHOULD/MAY only for implementer-facing invariants.
- No external people, repos, URLs, or paths unless the topic is specifically external.
- Cut filler. Do not invent milestones, owners, dates, or future work.

If scope or intent is unresolved after repo reads, ask one batched question and stop. Otherwise write both artifacts.

Final response exactly:
Artifact written: <path>
Plan written: <path>
