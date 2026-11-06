---
name: feature-flag-clean-up
description: Clean up a fully rolled-out feature flag while preserving its live behavior. Use when the user asks to remove, inline, or retire a feature flag or toggle.
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Remove the flag named in `$ARGUMENTS` or the conversation as a repo concept. Keep the live behavior and delete the dead path.

Establish the live value before editing. Use the user's statement, flag-service evidence, or committed defaults and overrides for every environment. Keep the branch that value selects. For inverted flags such as `disable_*` or kill switches, that is usually the off path. If the live value differs across environments or is not evidenced, ask before editing.

Search for the full key, enum, constant, and generated names, aliases, and string variants across production code, tests, config, docs, and fixtures. Do not stop at the first conditional.

Inline the live branch. Convert tests that cover both values into tests of the retained behavior, and delete tests that only cover the dead path. Remove the imports, helpers, types, config entries, and fixtures that become unused.

Repeat the search until no relevant references remain. Remote flag definitions in a flag service are not repo code. Leave archiving them to the user and say so. Verify the retained behavior through a targeted real path, and add a check if important behavior has none.

Final:
- Removed: `<flag>` (kept `<live value>` behavior)
- References: `<0 or remaining with reason>`
- Checks: `<command>` -> `<result>`, `reused — <prior proof and why still valid>`, or `not run — <reason>`
- Files: `<changed files>`
- Follow-up: `<remote flag archive or none>`
