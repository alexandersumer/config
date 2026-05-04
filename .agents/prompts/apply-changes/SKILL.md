---
name: apply-changes
description: Implement requested changes narrowly and consistently
argument-hint: "[optional: changes to apply]"
inputs:
  - name: changes
    label: Changes to apply
    description: Describe the changes to apply. Leave empty to apply changes from conversation context.
    type: string
    required: false
---

Apply the requested change. Use `changes`, else `$ARGUMENTS`, else the preceding conversation. If the change is still ambiguous after reading the relevant files, ask one concise question and stop.

Do not satisfy this by making a plausible-looking edit in isolation. The known failure mode is guessing the shape of the code from the request, adding a new abstraction, or changing nearby code because it feels cleaner.

Read the relevant files first. Match the existing naming, layering, error handling, test style, and comment density. Make the smallest correct change that preserves the surrounding design.

Required outcome:
- The requested behavior or text change is present.
- The diff contains only required lines and immediate consistency fixes.
- Existing call sites and tests still compile in the same way.
- No new dependency, abstraction, comment, or broad refactor appears unless the request itself requires it.

Final response:
- Changed: `<files>`
- Checks: `<command or not run: reason>`
- Notes: `<only if something remains unclear or blocked>`
