---
name: sync-main
description: Fetch latest main and merge it into the current branch
---

Stop if the working tree has uncommitted changes — tell the user to commit or stash first. Stop if already on the default branch.

Determine the default branch (main or master). Run `git fetch origin <default-branch>` to update the ref without switching branches.

Merge with `git merge origin/<default-branch>`. If the merge completes cleanly, done.

If there are conflicts: resolve to preserve the intent of the current branch while incorporating updates from the default branch. When the default branch has intentionally removed or simplified code (feature flags, dead code, deprecated APIs, temporary constructs) that was not introduced by the current branch, accept the removal — update the current branch's code to work without them.

After resolving, search all affected files for remaining conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`). Run the build if a build command is available. Stage all resolved files and complete the merge commit.
