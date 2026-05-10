---
name: review-branch
description: Use when reviewing a pull request, diff, branch, or any set of code changes.
---

# Review Branch

You do not review the code directly. Subagents do the reviewing in fresh context because session history biases the reviewer toward the author's framing and burns tokens on noise. Your job is dispatching and aggregating.

1. **Get the effective review diff.** Resolve the remote default branch from `origin/HEAD`, falling back to `origin/main` or `origin/master` only if needed. Review the union of committed branch changes, staged changes, unstaged changes, and untracked files: `git diff $(git merge-base <remote-default> HEAD)..HEAD`, `git diff --cached`, `git diff`, and `git ls-files --others --exclude-standard` rendered as new-file diffs. If the committed branch diff is empty but the working tree has staged, unstaged, or untracked changes, review those working-tree changes instead of stopping. If the effective review diff is empty, draft-only, formatter-only, version-bump-only, or already reviewed in this thread, stop with a one-line note.

2. **Read conventions yourself.** Find every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` whose directory is an ancestor of any changed file. Read them and paste the relevant convention text into reviewer prompts.

3. **Dispatch four fresh-context reviewers in parallel.** Each reviewer gets this prompt verbatim, with `{ROLE}`, `{DIFF}`, and `{CONVENTIONS}` filled in. No session context — only what you paste:

   > You are reviewing a code change as **{ROLE}**. Diff: {DIFF}. Conventions: {CONVENTIONS}. Return candidate issues with `file:line`, severity, and a one-paragraph rationale grounded in the diff or files you read. One issue = one root cause. Skip nitpicks, style, "consider also". If it is not a real defect or risk, drop it.

   Roles:
   - **Correctness** — logic errors, wrong returns, violated contracts
   - **Failure modes** — null/boundary inputs, races, swallowed errors, leaks, regressions in adjacent code the diff touches
   - **Security** — injection, auth, secrets, unsafe deserialization, missing validation
   - **Conventions** — rules scoped to changed files; skip what a linter catches

4. **Validate every candidate.** Use one fresh Task subagent per candidate issue. Each validator gets this prompt verbatim, with `{ISSUE}`, `{FILES}`, and `{CONVENTIONS}` filled in:

   > Issue: {ISSUE}. Relevant files in full: {FILES}. Conventions: {CONVENTIONS}. Confirm or refute. State concrete evidence — triggering input, line that executes wrong, rule violated. Score 0–100; anything you cannot demonstrate concretely scores under 80.

   Drop every candidate below 80.

5. **Report.** Dedupe by root cause, then rank Critical, High, Medium, Low. For each issue, include severity, `path:line`, what is wrong, why it matters, and the fix — one sentence each. End with **Ready to merge**, **Needs attention**, or **Needs work**. If zero candidates survive validation, say so in one line.

Never approve, never merge, never invent line numbers. Subagents see only what you paste.
