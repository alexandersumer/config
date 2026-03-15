---
name: resolve-conflict
description: Resolve merge conflicts preserving branch intent
---

Analyze the conflict markers and the intent of both the current branch and the incoming branch. Resolve conflicts to preserve the intent of the current branch while incorporating necessary updates from the incoming branch.

When the incoming branch has intentionally removed or simplified code (feature flags, dead code, deprecated APIs, temporary constructs) that was not introduced by the current branch, accept the removal rather than reintroducing it. Update the current branch's code to work without them.

After resolving, search the affected files for remaining conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`). Run the build if a build command is available.
