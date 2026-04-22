---
name: resolve-conflict
description: Resolve merge conflicts preserving branch intent
---

Read the conflict markers and the commit history of both branches to identify the intent on each side. Resolve conflicts so the current branch's intent is preserved while the incoming branch's updates are incorporated.

When the incoming branch intentionally removed or simplified code (feature flags, dead code, deprecated APIs, temporary constructs) that the current branch did not introduce, accept the removal. Adapt the current branch's code to work without it.

After resolving:
- Search every affected file for `<<<<<<<`, `=======`, `>>>>>>>`. Re-resolve any that remain.
- Run the build if a build command is available.

Acceptance criteria:
- No conflict markers remain in the working tree.
- Current branch's intent is intact.
- Code introduced or removed by the incoming branch is reflected, not reverted.
- Build passes if a build command exists.
