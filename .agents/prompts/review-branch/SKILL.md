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

Report every finding in these categories, regardless of how minor it seems within the category:
- Correctness bugs (logic errors, off-by-one, null/empty handling, race conditions, incorrect error handling).
- Security vulnerabilities (injection, auth/authz gaps, secret exposure, unsafe deserialization, SSRF, missing input validation).
- Architectural flaws (wrong layer, broken invariants, leaky abstractions, hidden coupling).
- Missing test coverage for any new or changed behavior.
- Behavioral rollout safety: any new or modified runtime behavior that affects users must be behind a feature flag or gradual rollout. If the codebase uses a rollout/feature-flag SDK, the control and replacement blocks must contain real old and new code paths so metrics, logging, and alerting capture real execution; flag literal-only or boolean-hack blocks. Config-only changes, infra updates, and pure refactors with no behavioral change are exempt.

Out of scope (do not report): formatting, naming preferences, comment wording, import order, or any purely subjective style point.

Output format — one finding per line:

```
<filename>:<line> <problem> -> <fix>
```

If nothing in scope was found, output exactly: `no issues found`.
