---
name: create-pull-request
description: Create a pull request for staged and unstaged changes. Use when the user asks to create, open, or prepare a PR for the current working tree.
---

Create a pull request for the staged and unstaged changes in the current repository.

Use repository conventions, but do not invent missing intent. Inspect the diff and use the existing branch when it is suitable. If no branch exists, derive one from the dominant change. Use the repository's configured remote and target branch instead of guessing.

Only include staged and unstaged changes already present in the working tree. Do not broaden scope, refactor opportunistically, or run unrelated cleanup. Commit what needs to be committed, publish the branch if needed, create or reuse the appropriate pull request, and finish with the PR URL plus a brief summary of what was done.

If there are no changes to ship, stop and say so.
