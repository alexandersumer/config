---
name: git-commit-push-to-origin
description: Create or update a pull request for staged and unstaged changes, publishing the branch to origin
---

Create or update a pull request for the staged and unstaged changes in the current repository, using `origin` as the publish remote when a push is needed.

Use repository conventions, but do not invent missing intent. Inspect the diff and use the existing branch when it is suitable. Use `origin` as the publish remote and the repository's configured default branch as the target instead of guessing.

Only include staged and unstaged changes already present in the working tree. Do not broaden scope, refactor opportunistically, or run unrelated cleanup. Commit what needs to be committed, publish the branch to origin if needed, create or reuse the appropriate pull request, and finish with the PR URL plus a brief summary of what was done.

If there are no changes to ship, stop and say so.
