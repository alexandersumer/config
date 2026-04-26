---
name: desloppify
description: Improve code quality with a dependency-free review/fix loop
argument-hint: "[optional: scan path]"
inputs:
  - name: path
    label: Scan path
    description: Directory to scan. Empty or repo-like phrases mean `.`.
    type: string
    required: false
---

Improve code quality using repo inspection and existing checks only.

Path: use `path` or `$ARGUMENTS`; treat empty or repo-like phrases (`repo`, `this repo`, `whole repository`, `this whole repository`) as `.`.

Skip obvious non-source dirs: `.git`, dependencies, build output, coverage, caches, generated output, worktrees. Ask before questionable skips.

Make a short queue of real issues: duplication, dead code, unclear names, brittle tests, missing checks, unsafe shell, fragile config, over-complex logic, stale comments, misleading docs.

Loop:
1. Fix the highest-impact item at the root cause.
2. Run the smallest relevant existing check.
3. Re-inspect and continue until done or blocked.

Do not add dependencies, suppressions, broad rewrites, deleted tests, or cosmetic-only churn.

Final response: path, skipped dirs, changed files, checks, remaining blockers.
