---
name: review-branch
description: Review a branch for bugs, security issues, architecture risks, and test gaps
argument-hint: "[optional: focus area]"
inputs:
  - name: focus_area
    label: Focus area
    description: Optional text to narrow the review scope. Leave empty to review the entire branch.
    type: string
    required: false
---

Review the current branch for material defects. Determine the base branch and inspect `git diff base...HEAD`. Narrow to `$ARGUMENTS` only if provided.

Do not satisfy this by listing preferences. The known failure mode is review noise: naming taste, formatting, speculative risks, or test suggestions that would not catch a realistic bug. Report only issues that can change correctness, security, maintainability, rollout safety, or reviewer confidence.

Look for:
- correctness bugs: logic errors, boundary mistakes, null/empty handling, races, incorrect error handling
- security issues: injection, auth/authz gaps, secret exposure, unsafe deserialization, SSRF, missing validation
- architectural risks: wrong layer, broken invariant, leaky abstraction, hidden coupling
- missing tests for important new or changed behavior that could plausibly regress
- rollout safety gaps for risky user-impacting runtime behavior when the repo has a normal rollout pattern

Do not report:
- formatting, import order, comment wording, naming preference, or subjective style
- risks already mitigated by the diff or surrounding code
- rollout demands for config-only changes, infra-only changes, or pure refactors

Output one finding per line:
`<filename>:<line> <problem> -> <fix>`

If nothing material is found, output exactly:
`no issues found`
