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

Remove fully rolled out feature flag `$ARGUMENTS`. Keep the enabled behavior. Delete the disabled behavior.

Do not satisfy this by deleting the obvious `if` and calling the build green. The known failure mode is leaving aliases, tests, config, docs, or dead disabled-path helpers behind. The cleanup is done only when the flag no longer exists as a concept in the repo and the retained enabled behavior is verified through a real path.

Before editing, search for:
- full flag key string
- enum, constant, generated, and config names
- obvious aliases and string variants
- production code, tests, config, docs, and fixtures

Apply the cleanup:
- Inline the enabled branch at every check.
- Delete disabled branches and tests that only assert disabled behavior.
- Remove the flag definition after references are gone.
- Remove now-unused imports, helpers, types, config, docs, and fixtures.
- Do not chase unrelated substring matches after context proves they are different concepts.

Verification:
- Repeat the searches and prove zero relevant references remain.
- Run targeted checks for the retained enabled behavior first.
- If no targeted check exists for important retained behavior, add or strengthen one.
- Run the broader build/test suite when available.

Final response:
- Removed: `<flag key>`
- References: `0 remaining` or `<remaining with reason>`
- Behavior verified: `<command>` -> `<result>`
- Files: `<changed files>`
