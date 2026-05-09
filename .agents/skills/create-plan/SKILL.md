---
name: create-plan
description: Write a spec and implementation plan artifact. Use when the user asks to create planning files, a proposal, design doc, or implementation plan.
---

Use `topic`, `$ARGUMENTS`, or the conversation context as the seed. Write the smallest useful artifact set: a primary spec/design/proposal and a companion implementation plan.

Do not write a planning-shaped document. Write down the decisions that matter. The artifacts must let an implementer start without rediscovering scope, behavior, decisions, files, tests, or acceptance.

First read repo conventions: planning dirs, nearby docs, README/CONTRIBUTING, and up to four relevant existing artifacts. Follow the local artifact style unless it produces filler.

Resolve paths before writing:
- primary: `out_path`, matching convention, or `.plan/<slug>.md`
- plan: repo convention or `<primary-stem>-plan.md`

The primary artifact must answer the questions an implementer would otherwise have to ask: what problem is being solved, what behavior changes, what design decisions are already made, what files or modules are implicated, what acceptance signal proves it works, and what is deliberately out of scope.

The implementation plan must break the work into reviewable end-to-end chunks. Each chunk must ship observable behavior or a usable capability through a real path, with a checkable signal. No helper-only chunks, no "write tests" chunks, no vague cleanup phases unless they unblock named behavior.

Use repo vocabulary. Do not invent owners, dates, milestones, external references, fake trade-offs, generic risks, or empty sections. If a section would say nothing useful, omit it.

If scope is unclear after reading, ask one batched question and stop.

Final: say only what was written and where.
