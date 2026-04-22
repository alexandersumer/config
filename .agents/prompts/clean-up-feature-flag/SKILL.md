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

Find every reference before editing: full key string, enum/constant name, short name, any aliases, and any string variants. Search production code, test code, configuration, and documentation.

Apply the cleanup:
- Replace each flag check with the enabled branch inlined; delete the disabled branch.
- Delete tests that exclusively cover the disabled behavior.
- Remove the flag definition from its enum/config once no references remain.
- Remove imports, helpers, and types left unused by the above.

Run the build and test suite. Fix any failures by correcting missed references or updating tests that asserted disabled behavior. Iterate until green.

Acceptance criteria:
- Zero references to the flag remain (verified by repeating the search).
- Build and tests pass.
- No behavioral changes beyond removing the disabled path.
