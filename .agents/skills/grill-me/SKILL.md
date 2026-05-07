---
name: grill-me
description: Stress-test a plan
argument-hint: "[optional: plan/design text or file path]"
inputs:
  - name: plan
    label: Plan or design
    description: Inline plan/design text, a file path (e.g. `.plan/foo.md`), or leave empty to infer from conversation context.
    type: string
    required: false
---

Stress-test `plan`, `$ARGUMENTS`, or the conversation plan.

Do not generate a questionnaire. Find the single unresolved decision most likely to change architecture, data model, boundaries, sequencing, verification, or irreversible cost.

If code can answer it, inspect code instead of asking.

Ask the question only after explaining why this is the pressure point. Give your recommended answer and the trade-off you are accepting. Then stop and wait.

Skip low-impact, convention-settled, or implementation-detail questions. If the plan is already decided enough to execute, say so and name the next action.

No template. No list of possible concerns. One hard question, your answer, then wait.
