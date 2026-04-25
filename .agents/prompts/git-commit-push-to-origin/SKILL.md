---
name: git-commit-push-to-origin
description: Commit all staged and unstaged changes with a conventional commit, then push the current branch to origin
---

Commit the staged and unstaged changes in the current repository using an appropriate conventional commit message, then push the current branch to `origin`.

Use the repository's normal conventions and available tools to inspect the changes and choose the conventional commit type, scope, and message. Include both staged and unstaged tracked changes, plus any relevant untracked files, unless the user explicitly says otherwise.

Do not create, switch, rename, or publish a new branch. Work only on the current branch. Do not create or update a pull request as part of this prompt.

Before staging or committing anything, determine the current branch and the remote default branch for `origin`. If the current branch is `main`, `master`, or the remote default branch, stop immediately with an error-style message explaining that this prompt refuses to commit or push directly to a default branch. Do not stage files, create a branch, commit, push, or attempt a workaround.

Work autonomously after the branch safety check passes: inspect the working tree, stage the intended changes, commit them with a conventional commit message, push the current branch to `origin`, and finish with a brief summary of what was committed and pushed.

If there are no changes to commit, stop and say so.
