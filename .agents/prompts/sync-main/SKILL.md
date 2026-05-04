---
name: sync-main
description: Merge latest default branch
---

Fetch the latest default branch and merge it into the current branch.

Stop if the working tree is dirty or the current branch is the default branch. Do not stash, discard, or guess.

Resolve remote/default branch from upstream HEAD, falling back to common names only if needed. Always fetch before merging.

Run `git merge <remote>/<default-branch>` and inspect status.

On conflicts, preserve current branch intent while incorporating default-branch changes. Accept default-branch removals of flags, dead code, deprecated APIs, or temporary constructs this branch did not introduce.

Search for conflict markers. Run build if available. Stage resolved files and complete the merge commit.

Final:
- Synced: `<branch>` with `<remote>/<default>`
- Merge: `<fast-forward|merge commit|conflicts resolved>`
- Checks: `<command>` -> `<result>`
