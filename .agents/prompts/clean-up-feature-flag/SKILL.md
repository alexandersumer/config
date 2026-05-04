---
name: clean-up-feature-flag
description: Remove a rolled-out flag
argument-hint: "[flag key]"
inputs:
  - name: flag_key
    label: Flag key
    description: The feature flag key to remove.
    type: string
    required: true
---

Remove fully rolled out flag `$ARGUMENTS`. Keep enabled behavior. Delete disabled behavior.

Do not stop at the first conditional. Remove the flag as a repo concept: checks, aliases, constants, config, tests, docs, fixtures, dead helpers.

Search first for full key, enum/constant/generated names, aliases, and string variants across production, tests, config, docs, and fixtures.

Inline enabled branches. Delete disabled-only tests. Remove unused imports, helpers, types, and config.

Repeat searches until no relevant references remain. Verify retained behavior through a targeted real path; add one if important behavior has no check. Run broader tests when available.

Final:
- Removed: `<flag>`
- References: `<0 or remaining with reason>`
- Checks: `<command>` -> `<result>` or `not run — <reason>`
- Files: `<changed files>`
