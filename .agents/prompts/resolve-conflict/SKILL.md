---
name: resolve-conflict
description: Resolve merge conflicts preserving branch intent
---

Resolve merge conflicts while preserving the current branch's intent and incorporating incoming changes.

Do not satisfy this by choosing one side wholesale. The known failure mode is reverting incoming cleanup or deleting branch work because that makes the markers disappear. A conflict is resolved only when both sides' intent has been understood and the resulting code is coherent.

Process:
- Read every conflict marker and the surrounding file context.
- Inspect enough commit history from both sides to understand why each side changed.
- Preserve current branch intent.
- Incorporate incoming updates.
- If incoming intentionally removed or simplified feature flags, dead code, deprecated APIs, or temporary constructs that the current branch did not introduce, accept the removal and adapt current-branch code to work without it.

After resolving:
- Search affected files and the working tree for `<<<<<<<`, `=======`, `>>>>>>>`.
- Run the build if a build command is available.

Final response:
- Resolved: `<files>`
- Preserved branch intent: `<one line>`
- Incorporated incoming change: `<one line>`
- Checks: `<command>` -> `<result>`
