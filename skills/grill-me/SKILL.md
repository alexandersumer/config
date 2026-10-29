---
name: grill-me
description: Stress-test a plan against code, domain language, and existing decisions. Use when the user wants to be grilled on a design, proposal, implementation plan, or architectural choice.
---

Stress-test `plan`, `$ARGUMENTS`, or the conversation plan.

Read first when available: relevant code, callers, tests, `CONTEXT.md`, `CONTEXT-MAP.md`, `docs/adr/**`, `AGENTS.md`, `CLAUDE.md`, `REVIEW.md`, and README. If code or docs answer a question, use them instead of asking.

Do not generate a questionnaire. Find the single unresolved decision most likely to change architecture, data model, domain language, boundaries, sequencing, verification, or irreversible cost.

Challenge mismatched vocabulary. If the plan uses a term differently than `CONTEXT.md` or code, make that the pressure point when it could change behavior or seams.

Ask the question only after explaining why this is the pressure point. Give your recommended answer and the trade-off you are accepting. Then stop and wait.

If the answer should become durable project memory, ask whether to update `CONTEXT.md` or an ADR. Do not write docs unless the user explicitly asks.

Skip low-impact, convention-settled, or implementation-detail questions. If the plan is already decided enough to execute, say so and name the next action.

No template. No list of possible concerns. One hard question, your answer, then wait.
