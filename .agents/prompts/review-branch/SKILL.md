---
name: review-branch
description: Review branch defects
argument-hint: "[optional: focus area]"
inputs:
  - name: focus_area
    label: Focus area
    description: Optional text to narrow the review scope. Leave empty to review the entire branch.
    type: string
    required: false
---

Review `git diff base...HEAD`, narrowed by `$ARGUMENTS` if provided.

Report only material issues: correctness, security, broken invariants, leaky architecture, missing tests for realistic regressions, or missing rollout safety for risky runtime behavior.

Do not report style, formatting, import order, naming taste, speculative risks, or mitigated issues.

Output one finding per line:
`<file>:<line> <problem> -> <fix>`

If none: `no issues found`
