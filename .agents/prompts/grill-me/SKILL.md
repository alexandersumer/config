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

<intent>
Stress-test a plan or design by asking focused questions one at a time until the important decisions, dependencies, and trade-offs are clear.
</intent>

<constraints>
- Ask one question at a time. Wait for the answer before the next question.
- For each question, provide your recommended answer upfront so you model what "good" looks like.
- If a question can be answered by exploring the codebase (existing patterns, architecture, dependencies), do that instead of asking.
- Prefer the highest-leverage unresolved decision first. Do not exhaustively enumerate branches that are obvious, low-impact, or already settled by repo conventions.
- Stop when there are no remaining open questions or when you reach a decision that conflicts with the plan's stated intent — surface the conflict explicitly.
</constraints>

<acceptance_criteria>
- Important architectural choices have a stated reason or constraint backing them.
- Dependencies between decisions are named (e.g., "this choice depends on the decision about X").
- The user confirms agreement on each branch before moving on.
- If a conflict emerges between the plan and a decision, it is surfaced and resolved or explicitly deferred.
- At the end, you summarize the resolved design tree in a form the user can reference later.
</acceptance_criteria>

<context>
Start with the plan/design provided in $ARGUMENTS, or infer it from the preceding conversation. Read it end-to-end first before asking any questions. If it is ambiguous or incomplete on a major structural choice, start there.
</context>

<output_format>
Example opening:

```
Plan: <source>

**Q1: [decision area]**

Recommended: <your suggestion>

Your turn: <wait for answer>
```

At the end, a summary like:

```
Resolved design tree:
- [decision] -> [rationale] (confirmed)
- [decision] -> [rationale] (confirmed)
- [decision] -> [rationale] (confirmed)

Unresolved:
- [open question]: <blocking reason>

Next: <suggest what to do with this grilled design>
```
</output_format>
