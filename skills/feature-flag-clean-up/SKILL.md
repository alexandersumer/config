---
name: feature-flag-clean-up
description: Clean up a rolled-out feature flag while preserving enabled behavior
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Remove fully rolled out flag `flag_key` or `$ARGUMENTS`. Keep enabled behavior. Delete disabled behavior.

Do not stop at the first conditional. Remove the flag as a repo concept: checks, aliases, constants, config, tests, docs, fixtures, dead helpers.

Search first for full key, enum/constant/generated names, aliases, and string variants across production, tests, config, docs, and fixtures.

Inline enabled branches. Delete disabled-only tests. Remove unused imports, helpers, types, and config.

Repeat searches until no relevant references remain. Verify retained behavior through a targeted real path; add one if important behavior has no check. Reuse proof when valid; run broader tests only when the proof policy justifies them.

Final:
- Removed: `<flag>`
- References: `<0 or remaining with reason>`
- Checks: `<command>` -> `<result>`, `reused — <prior proof and why still valid>`, or `not run — <reason>`
- Files: `<changed files>`
