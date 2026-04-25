---
name: git-commit-push
description: Commit staged and unstaged changes with a Conventional Commit message, then push the current branch to its upstream
---

Commit the current staged and unstaged changes, then push the current branch to its configured upstream.

Steps:
1. Inspect `git status`, the current branch, the staged diff, and the unstaged diff.
2. If the current branch is `main` or `master`, stop with an error. Do not commit or push from the default branch.
3. If there are no changes, stop and say so.
4. Stage all intended working-tree changes exactly as they are. Do not edit files, broaden scope, clean up unrelated code, or create extra changes.
5. Write one Conventional Commit subject that describes the dominant change: `<type>[optional scope]: <description>`.
6. Commit with that subject. Add a short body only when the subject alone would hide important context.
7. Push to the current branch's upstream. If no upstream exists, push to a branch of the same name on `origin` and set upstream.

Rules:
- Use the current branch. Do not create, rename, switch, merge, rebase, or reset branches.
- Do not create or update a pull request.
- Do not run formatters, tests, builds, or generators unless the user explicitly asked.
- If the diff contains unrelated changes that cannot honestly fit one commit subject, stop and ask how to split them.
- If pushing is rejected, report the rejection and stop; do not force-push.

Final response:
- Branch pushed
- Commit hash and subject
- Files committed
- Push result
