---
name: review-solo
description: Direct code-change review without subagents. Use for ordinary, direct, inline, single-agent, or no-subagent review of code/config/test changes. Use review-deep when the user asks for deep, thorough, multi-agent, high-confidence, or heavyweight review.
---

# Review Solo

Review the code directly in this session. Do not invoke subagents. Preserve the same quality bar as `review-deep` by doing explicit, sequential review passes and validating every finding against concrete code, behavior, or conventions.

1. **Get the effective review diff automatically.** Do not require a PR, explicit scope, or committed branch changes. Resolve the remote default branch from `origin/HEAD`, falling back to `origin/main` or `origin/master` only if needed; if no remote default exists, omit only the committed-branch part. Build one effective diff from the union of: committed branch changes with `git diff $(git merge-base <remote-default> HEAD)..HEAD`, staged changes with `git diff --cached`, unstaged changes with `git diff`, and untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs. Always include staged, unstaged, and untracked changes even when the committed branch diff exists. If `focus_area` or `$ARGUMENTS` is provided, use it only to narrow this discovered diff. If the effective review diff is empty, draft-only, formatter-only, version-bump-only, or already reviewed in this thread, stop with a one-line note.

2. **Read conventions and real code context.** Find every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` whose directory is an ancestor of any changed file. Read them. Then read enough surrounding code to understand entry points, callers, state/model changes, public contracts, tests, configs, and failure paths touched by the diff. Keep this bounded to the smallest context needed to validate findings.

3. **Run four direct review passes.** Do not report during the passes. Collect candidate issues only when they have a concrete root cause and plausible user/system impact:
   - **Correctness** — logic errors, wrong returns, violated contracts, stale state, wrong ordering, broken data flow.
   - **Failure modes** — null/boundary inputs, races, swallowed errors, unclear or misleading error messages, missing diagnostic context, noisy or low-signal logging, leaks, retries, partial failures, regressions in adjacent touched code.
   - **Security** — injection, auth/permission gaps, secrets or sensitive data in logs/errors, unsafe deserialization, missing validation, trust-boundary mistakes.
   - **Conventions** — rules scoped to changed files, local patterns, ownership boundaries; skip what a linter catches. For changed tests, flag low-value or redundant tests only when they add maintenance cost without a named realistic regression signal or duplicate stronger existing coverage.

4. **Validate every candidate yourself.** For each candidate, inspect the relevant file/function/caller/test path in enough detail to confirm or refute it. Prefer concrete execution paths, triggering input, violated rule, or a targeted command. Drop anything you cannot demonstrate concretely. Never keep a finding because it sounds plausible.

5. **Report.** Dedupe by root cause, then rank Critical, High, Medium, Low. For each issue, include severity, `path:line` if known, what is wrong, why it matters, and the minimal fix — one concise paragraph each. If the review surface is too large to validate directly, say `Review limited` and name the unreviewed surface as residual risk. If no validated findings remain, say in one line that no material issue was found and name the reviewed surface.

Never approve, never merge, never invent line numbers. Do not use subagents.
