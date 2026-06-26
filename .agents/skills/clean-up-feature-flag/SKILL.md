---
name: clean-up-feature-flag
description: Remove a rolled-out flag
---

## Validation reuse and check scope

Before running a slow, broad, external, stateful, or CI-equivalent command, check whether this conversation or current-SHA CI/artifacts already contain usable proof. Reuse prior passing evidence instead of rerunning only when it is visible, ran after the last relevant edit, covers the same command/scenario and behavior, edge case, or public boundary, and no touched file, config, dependency, fixture, generated output, runtime state, or environment assumption it depends on changed afterward. If uncertain, run the narrowest freshness check that resolves the uncertainty before escalating.

Default to the narrowest honest proof. Run broader suites, full builds, CI reruns, or live/E2E flows only when required by blast radius, merge/release policy, changed shared API/schema/build/test infrastructure/dependencies/auth/security/persistence/concurrency, merge/conflict integration risk, missing targeted seams, or because the broad command is the only proof that covers the behavior.

Final reports must distinguish reused proof, newly run commands, and checks intentionally not run.

Remove fully rolled out flag `flag_key` or `$ARGUMENTS`. Keep enabled behavior. Delete disabled behavior.

Do not stop at the first conditional. Remove the flag as a repo concept: checks, aliases, constants, config, tests, docs, fixtures, dead helpers.

Search first for full key, enum/constant/generated names, aliases, and string variants across production, tests, config, docs, and fixtures.

Inline enabled branches. Delete disabled-only tests. Remove unused imports, helpers, types, and config.

Repeat searches until no relevant references remain. Verify retained behavior through a targeted real path; add one if important behavior has no check. Reuse fresh prior proof when valid; run broader tests only when the validation policy justifies them.

Final:
- Removed: `<flag>`
- References: `<0 or remaining with reason>`
- Checks: `<command>` -> `<result>`, `reused — <prior proof and why still valid>`, or `not run — <reason>`
- Files: `<changed files>`
