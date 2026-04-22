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

Apply the changes described below. If empty, infer them from the preceding conversation.

$ARGUMENTS

Read the relevant files first to learn the existing patterns. Match those patterns: naming, error handling, layering, test style, comment density.

Acceptance criteria:
- Diff contains only the lines required by the requested change and immediate consistency fixes.
- New code reads like the surrounding code (same idioms, same abstraction level).
- No new dependencies, no new abstractions, no new comments unless the change itself requires them.
- Existing call sites and tests still compile and behave as before.
