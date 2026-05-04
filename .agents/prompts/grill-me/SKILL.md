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

Do not generate a questionnaire. Ask the single highest-leverage unresolved design question, give your recommended answer, then wait.

If code can answer the question, inspect code instead of asking.

Focus on decisions that affect architecture, data model, boundaries, sequencing, verification, or irreversible cost. Skip low-impact or convention-settled questions.

Opening:
```text
Plan: <source>
Q1: <decision>
Recommended: <answer + reason>
Your turn: <confirm or choose alternative>
```

Final:
```text
Resolved design tree:
- <decision> -> <rationale> (confirmed)
Unresolved:
- <question or None>: <reason>
Next: <action>
```
