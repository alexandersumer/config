---
name: git-publish
description: Commit current changes with a conventional commit, push the branch, and create or update the pull request.
---

Commit the current staged, unstaged, and relevant untracked changes, push the working branch, and create or update its pull request.

Inspect the diff, branch state, remotes, recent commits, and existing pull requests to choose the branch, remote, target branch, pull request title, and pull request description. Use available SCM/code-review tools when they are safer than raw git/provider CLI commands.

Before committing, choose one canonical Conventional Commit subject for the new commit. For a new pull request, choose a branch-level Conventional Commit subject for the pull request title. For an existing pull request, preserve the existing title and description unless the user explicitly asks to update pull request metadata or the existing metadata is empty/provider-generated.

Subject rules:
- Match exactly: `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`.
- Format: `<type>[optional scope][!]: <lowercase imperative description>`.
- No trailing period.
- Do not use a sentence-case summary, branch name, issue title, issue-key prefix, or SCM/provider default.
- If the commit subject fails validation, fix it before any commit or push. If creating a pull request, also validate the new pull request title before creation.

Workflow:
1. Inspect the repository state: current branch, remotes, default branch, staged changes, unstaged changes, untracked files, and whether a pull request already exists for the branch.
2. If there are no staged, unstaged, or relevant untracked changes to commit, stop and say so.
3. If the current branch is `main`, `master`, or the remote default branch, create a new appropriately named branch before committing. Do not commit directly to the default branch.
4. Include staged changes, unstaged tracked changes, and relevant untracked files unless the user explicitly excludes something.
5. Commit using the validated subject as the commit message first line. Add a body only when useful.
6. Push the branch. If a pull request already exists, reuse it without changing its title or description unless explicitly requested. If creating a new pull request, set the title to the validated branch-level subject and add an appropriate branch-level description.
7. Finish with the pull request URL and a brief status summary. Do not treat the chat summary as the commit subject or pull request title.
