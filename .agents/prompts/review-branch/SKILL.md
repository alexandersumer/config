---
name: review-branch
description: Review for bugs, security, architecture, and test coverage (no style nits)
argument-hint: "[optional: focus area]"
inputs:
  - name: focus_area
    label: Focus area
    description: Optional text to narrow the review scope. Leave empty to review the entire branch.
    type: string
    required: false
---

Determine the base branch. Get the cumulative branch diff with three-dot syntax: `git diff base...HEAD`.

Scope: review the entire diff, or narrow to `$ARGUMENTS` if provided.

Report material findings in these categories. Skip theoretical, low-impact, or already-mitigated issues unless they are likely to affect correctness, security, maintainability, or reviewability:
- Correctness bugs (logic errors, off-by-one, null/empty handling, race conditions, incorrect error handling).
- Security vulnerabilities (injection, auth/authz gaps, secret exposure, unsafe deserialization, SSRF, missing input validation).
- Architectural flaws (wrong layer, broken invariants, leaky abstractions, hidden coupling).
- Missing test coverage for important new or changed behavior that could plausibly regress.
- Behavioral rollout safety: user-impacting runtime behavior should follow the repo's normal rollout pattern. Flag missing or fake rollout wrappers only when the change is risky enough to need gradual exposure. Config-only changes, infra updates, and pure refactors with no behavioral change are exempt.

Out of scope (do not report): formatting, naming preferences, comment wording, import order, or any purely subjective style point.

Output format — one finding per line:

```
<filename>:<line> <problem> -> <fix>
```

If nothing in scope was found, output exactly: `no issues found`.
