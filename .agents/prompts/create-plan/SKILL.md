---
name: create-plan
description: Write a spec and plan
argument-hint: "[optional: feature/topic, problem statement, or path to seed notes/artifact]"
inputs:
  - name: topic
    label: Topic or seed
    description: One-line feature/topic, problem statement, or path to seed notes/artifact. Leave empty to infer from conversation context.
    type: string
    required: false
  - name: out_path
    label: Output path
    description: Optional path for the primary artifact. Leave empty to mirror repo artifact conventions.
    type: string
    required: false
---

Write two artifacts: a primary spec/design/proposal and a companion implementation plan.

Do not write generic planning prose. The artifacts must let an implementer start without rediscovering scope, decisions, files, tests, or acceptance.

First read repo conventions: planning dirs, nearby docs, README/CONTRIBUTING, and up to four relevant existing artifacts. Emit at most four `Conventions: <path> for <aspect>` lines.

Resolve paths before writing:
- primary: `out_path`, matching convention, or `.plan/<slug>.md`
- plan: repo convention or `<primary-stem>-plan.md`
- emit `Artifact path:` and `Plan path:`

Primary artifact must cover:
- problem, goals, non-goals
- decisions with rationale
- implementable design
- changed files, real or `NEW`
- falsifiable acceptance criteria
- out of scope

Plan must cover:
- strategy
- 2-5 reviewable end-to-end chunks
- files and acceptance signal per chunk
- dependencies
- targeted and full verification
- deferred work or `None`

Use repo vocabulary. Do not invent owners, dates, milestones, external references, or fake trade-offs.

If scope is unclear after reading, ask one batched question and stop.

Final exactly:
Artifact written: <path>
Plan written: <path>
