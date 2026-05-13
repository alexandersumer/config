---
name: resolve-conflict
description: Resolve merge conflicts
register_cmd: true
---

Resolve conflicts while preserving current branch intent and incorporating incoming changes.

Do not choose one side wholesale just to remove markers.

Read conflict context and enough history from both sides. If incoming removed feature flags, dead code, deprecated APIs, or temporary constructs this branch did not introduce, accept the removal and adapt branch code.

Search for `<<<<<<<`, `=======`, `>>>>>>>` after editing. Run the build if available.

Final:
- Resolved: `<files>`
- Preserved: `<branch intent>`
- Incorporated: `<incoming change>`
- Checks: `<command>` -> `<result>` or `not run — <reason>`
