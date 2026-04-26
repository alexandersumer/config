---
name: desloppify
description: Improve code quality with a dependency-free desloppify-style review/fix loop
argument-hint: "[optional: scan path]"
inputs:
  - name: path
    label: Scan path
    description: Directory to scan. Empty or repo-like phrases mean `.`.
    type: string
    required: false
---

Improve code quality using only repo inspection and existing checks.

Path: use `path`; else use `$ARGUMENTS`; treat empty, `repo`, `repository`, `this repo`, `this repository`, `whole repo`, `whole repository`, and `this whole repository` as `.`.

Inspect the path, skipping obvious non-source dirs (`.git`, `node_modules`, `vendor`, `dist`, `build`, `coverage`, caches, generated output, worktrees). Ask before skipping anything questionable.

Build a short queue of real issues from the code: duplication, unclear naming, dead code, brittle tests, missing checks, unsafe shell, fragile config, over-complex logic, stale comments, and docs that mislead usage.

Loop until done or blocked:
1. Pick the highest-impact queue item.
2. Fix the root cause, not symptoms.
3. Run the smallest relevant existing check.
4. Re-inspect the touched area and continue.

Do not add dependencies, suppressions, broad rewrites, deleted tests, or cosmetic-only churn.

Final response: path, skipped dirs, changed files, checks, remaining queue/blockers.
