---
name: create-chat-plan
description: Create a lightweight implementation-ready plan directly in chat. Use when planning should be robust, interrogated, and verified upfront without writing plan files.
---

Use `topic`, else `$ARGUMENTS`, else infer the planning target from the conversation, active design, current branch, or referenced files. Produce the plan in chat only. Do not create markdown files, do not edit code, and do not invoke subagents unless the user explicitly asks for deeper research.

First read enough context to avoid generic planning: relevant README/CONTRIBUTING/AGENTS instructions, nearby docs, entry points, interfaces, representative callers, tests, existing plans, and current diffs when they matter. Keep this bounded; the goal is a high-confidence lightweight plan, not a durable design artifact.

If the target, success criteria, or constraints are unclear after context reading, ask one concise batched question and stop. Otherwise, state the assumptions you are making and proceed. Prefer reversible assumptions for routine details; do not ask questions that local code or docs can answer.

Define acceptance before implementation to avoid biased verification. Acceptance criteria must describe externally observable behavior, compatibility boundaries, and important edge cases before any implementation steps. The verification plan must name the narrowest checks that prove those criteria, the regression signal that would fail without the change when applicable, and any broader checks justified by blast radius. If no good automated seam exists, name the gap and the manual proof required.

Interrogate the plan before presenting it. Run a brief pre-mortem against missing data/API/schema compatibility, migration and rollback needs, concurrency or idempotency risks, permissions/security concerns, user-visible behavior, observability, test seams, ownership boundaries, and hidden coupling. Include only risks that are concrete for the scoped work, each paired with the design response.

Break implementation into reviewable end-to-end chunks. Each chunk should deliver observable behavior, reduce uncertainty, or unlock the next slice. Avoid helper-only phases, vague cleanup, "write tests" as a separate phase, or broad refactors unless they are the smallest path to the acceptance criteria. Prefer the smallest coherent plan that can pass the verification plan.

Return this structure, omitting empty sections:

## Goal
One or two sentences describing the outcome and user/system value.

## Acceptance criteria
- Observable behavior or artifact that must be true when done.
- Important edge cases, compatibility expectations, or non-goal boundaries.

## Verification plan
- Targeted checks, tests, scripts, or manual proof to run before completion.
- Regression signal expected to fail before and pass after, when applicable.
- Broader checks only when the change crosses enough surface area to justify them.

## Assumptions and open questions
- Assumptions used to keep planning moving.
- Questions that must be answered before implementation, if any.

## Risks and design pressure
- Concrete risks and the design response for each.

## Plan
1. End-to-end implementation slice with its checkable signal.
2. Next slice, only if needed.
3. Cleanup/docs/migration only when tied to acceptance.

## Out of scope
- Explicit exclusions and tempting follow-ups not included in this plan.

Final line: ask whether to implement the plan, revise it, or turn it into a durable `create-plan` artifact.
