---
description: Resolve merge conflicts preserving branch intent
---

Analyze the conflict markers and the intent of both the current branch and the incoming branch. Resolve conflicts to preserve the intent of the current branch while incorporating necessary updates from the incoming branch.

If the incoming branch has removed feature flags, experiments, or other temporary constructs as part of cleanup, do not reintroduce them. Accept the removal and update the current branch's code to work without them.
