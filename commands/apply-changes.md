---
name: apply-changes
description: Implement requested changes with surgical precision
argument-hint: "[optional: changes to apply]"
inputs:
  - name: changes
    label: Changes to apply
    description: Describe the changes to apply. Leave empty to apply changes from conversation context.
    type: string
    required: false
---

Read the relevant code to understand existing patterns and context. Apply changes with surgical precision. Use the changes described below if provided; otherwise, infer what to apply from context.

$ARGUMENTS

Do not over-engineer. Keep changes robust with no negative side effects on surrounding code. Ensure consistency with existing patterns. Avoid unnecessary comments.
