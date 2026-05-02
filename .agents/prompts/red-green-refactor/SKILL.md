---
name: red-green-refactor
description: Drive a small implementation with a disciplined test-first red/green/refactor loop. Use when adding or fixing behavior where tests can be written, when the user asks for TDD, test-first, regression tests, minimal implementation, or wants to prevent speculative overbuilding.
argument-hint: "[optional: behavior, bug, file, or scope]"
inputs:
  - name: scope
    label: Behavior or scope
    description: Optional behavior, bug, file, or scope to implement with a red-green-refactor loop. Leave empty to infer from conversation.
    type: string
    required: false
---

<intent>
Implement one small behavior at a time by first making the desired behavior executable, then making it pass with the smallest correct change, then improving the design while preserving the test signal.
</intent>

<workflow>
1. Resolve the behavior from `$ARGUMENTS`, conversation, issue text, current diff, or nearby failing tests. If unclear, ask one focused question.
2. Read the production entry point and nearest tests to learn repo conventions before editing.
3. Red: add or strengthen a focused test that fails for the missing/buggy behavior. Prefer public behavior over private implementation details.
4. Green: make the smallest production change that passes the test without weakening assertions, skipping tests, or adding fake fallbacks.
5. Refactor: improve names, boundaries, duplication, or structure only where the green implementation exposed real complexity. Keep behavior unchanged.
6. Run the narrow relevant test first, then a broader check if the touched area warrants it.
</workflow>

<constraints>
- Keep each loop small enough to review.
- Do not write tests for trivial boilerplate just to follow the ritual.
- Do not overbuild abstractions before a second real use appears.
- If test-first is impractical, explain why and create the closest executable regression signal before or alongside the fix.
</constraints>

<output_format>
Use this concise structure:

```text
Behavior: <behavior>

Red:
- <test added/changed> — expected failure: <why it fails before fix>

Green:
- <minimal production change>

Refactor:
- <cleanup performed or "none needed">

Verification:
- `<command>` -> <result>

Next:
- <next smallest behavior or done>
```
</output_format>
