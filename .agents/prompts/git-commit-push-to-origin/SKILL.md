---
name: git-commit-push-to-origin
description: Create or update a pull request for staged and unstaged changes, publishing the branch to origin
---

Create or update a pull request for the staged and unstaged changes in the current repository, using `origin` as the publish remote when a push is needed.

Use the repository's normal conventions and available tools to decide the branch, commit message, target branch, PR title, and PR description. Prefer integrated SCM/code-review tools when they can safely perform an operation; otherwise use standard git and provider CLI workflows.

Work autonomously: inspect the changes, commit what needs to be committed, publish the branch to origin if needed, create or reuse the appropriate pull request, and finish with the PR URL plus a brief summary of what was done.

If there are no changes to ship, stop and say so.
