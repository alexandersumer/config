---
name: clean-up-feature-flag
description: Remove a fully rolled out feature flag and verify checks pass
argument-hint: "[flag key]"
inputs:
  - name: flag_key
    label: Flag key
    description: The feature flag key to remove.
    type: string
    required: true
---

Remove fully rolled out feature flag `$ARGUMENTS`. Keep the enabled behavior; delete the disabled paths.

Find references before editing: full key string, enum/constant name, obvious aliases, and common string variants. Search production code, tests, configuration, and docs, but do not chase unrelated substring matches once context proves they are different concepts.

Apply the cleanup:
- Replace each flag check with the enabled branch inlined; delete the disabled branch.
- Delete tests that exclusively cover the disabled behavior.
- Remove the flag definition from its enum/config once no references remain.
- Remove imports, helpers, and types left unused by the above.

Run targeted checks first, then the build/test suite when available. Fix failures by correcting missed references or updating tests that asserted disabled behavior. Iterate until green or clearly blocked.

Acceptance criteria:
- Zero references to the flag remain (verified by repeating the search).
- Build and tests pass.
- No behavioral changes beyond removing the disabled path.
