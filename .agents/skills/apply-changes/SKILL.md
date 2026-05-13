---
name: apply-changes
description: Apply a narrow requested code change. Use when the user asks to edit, tweak, add, remove, or rename specific behavior without a broader plan.
register_cmd: true
---

Apply `changes`, else `$ARGUMENTS`, else the conversation request.

Do not guess from the prompt. Read the relevant files first, then make the smallest correct edit that matches existing naming, layering, error handling, tests, and comment density.

Do not add dependencies, abstractions, broad refactors, or explanatory comments unless the request requires them.

If the change is still ambiguous after reading context, ask one question and stop.

Final:
- Changed: `<files>`
- Checks: `<command or not run: reason>`
