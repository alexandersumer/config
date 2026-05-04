---
name: grill-me
description: Stress-test a plan or design with focused questions until the important decisions are clear
argument-hint: "[optional: plan/design text or file path]"
inputs:
  - name: plan
    label: Plan or design
    description: Inline plan/design text, a file path (e.g. `.plan/foo.md`), or leave empty to infer from conversation context.
    type: string
    required: false
---

Stress-test the plan or design until the important decisions, dependencies, and trade-offs are clear.

Do not satisfy this by generating a questionnaire. The known failure mode is asking many generic questions that make the user do the design work. Ask the highest-leverage unresolved question, give your recommended answer, and wait.

Start from `plan`, else `$ARGUMENTS`, else conversation context. If the input is a path, read it end to end. If a question can be answered by inspecting the codebase, inspect instead of asking.

Rules:
- Ask one question at a time.
- Lead with your recommended answer and rationale.
- Prefer decisions that affect architecture, data model, boundaries, sequencing, verification, or irreversible cost.
- Skip obvious, low-impact, or repo-convention-settled branches.
- Stop when the design tree is clear or when a decision conflicts with the plan's stated intent.

Opening format:
```text
Plan: <source>

Q1: <decision area>
Recommended: <answer + reason>
Your turn: <specific confirmation or alternative requested>
```

Final summary:
```text
Resolved design tree:
- <decision> -> <rationale> (confirmed)

Unresolved:
- <open question or None>: <blocking reason>

Next: <what to do with the grilled design>
```
