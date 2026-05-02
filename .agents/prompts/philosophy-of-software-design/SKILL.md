---
name: philosophy-of-software-design
description: Apply John Ousterhout-style software design judgment to reduce complexity, improve module depth, sharpen interfaces, and avoid tactical coding. Use when reviewing a design, planning an implementation, refactoring, evaluating architecture, or when the user mentions complexity, deep modules, shallow modules, interfaces, abstractions, decomposition, maintainability, or A Philosophy of Software Design.
argument-hint: "[optional: design, diff, file, plan, or scope]"
inputs:
  - name: scope
    label: Design scope
    description: Optional design, diff, file, plan, or scope to evaluate. Leave empty to infer from conversation or current branch.
    type: string
    required: false
---

<intent>
Evaluate software through the lens of reducing long-term complexity. Prefer simple interfaces hiding substantial implementation depth, cohesive boundaries, and changes that make future work easier to understand.
</intent>

<workflow>
1. Resolve the scope from `$ARGUMENTS`, recent conversation, a planning artifact, a file path, or the current branch diff. If the scope is ambiguous, ask one focused question.
2. Read enough code/design context to understand module boundaries, callers, data flow, and existing conventions before judging.
3. Identify complexity sources: information leakage, shallow modules, unclear names, special cases, hidden coupling, duplication of knowledge, tactical patches, and premature/generalized abstractions.
4. Recommend the smallest design move that reduces complexity without broadening scope.
</workflow>

<principles>
- Complexity is the enemy: optimize for understandable future changes, not just passing today’s task.
- Deep modules are better than shallow modules: a good interface is simple while hiding meaningful internal complexity.
- Interfaces matter more than implementations: improve what callers must know, not only how internals are written.
- Pull complexity downward when it simplifies many callers; do not leak implementation details upward.
- Prefer general-purpose mechanisms only when they remove real repeated special cases; otherwise keep the design concrete.
- Avoid tactical coding: do not add patches, flags, wrappers, or conditionals that solve the immediate symptom while making the system harder to reason about.
</principles>

<output_format>
Use this concise structure:

```text
Scope: <scope>
Design read: <key files/artifacts>

Complexity risks:
- <risk> — why it increases future cognitive load — evidence: <file/ref>

Recommended design move:
- <smallest change that reduces complexity>

Trade-offs:
- <what gets simpler>
- <what cost or risk remains>

Verdict: <keep as-is | adjust before implementation | refactor now | split/defer>
```

If the design is already good, say so and name the design properties that make it good.
</output_format>
