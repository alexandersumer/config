---
name: desloppify
description: Run Desloppify's scan/next/resolve loop to improve code quality
argument-hint: "[optional: scan path]"
inputs:
  - name: path
    label: Scan path
    description: Directory to scan. Empty or repo-like phrases mean `.`.
    type: string
    required: false
---

Run Desloppify and follow its queue.

Path: use `path`; else use `$ARGUMENTS`; treat empty, `repo`, `repository`, `this repo`, `this repository`, `whole repo`, and `whole repository` as `.`.

First run `command -v desloppify`. If missing, reply only: `desloppify is not available on PATH.`

Before scanning, exclude obvious non-source dirs only (`.git`, `node_modules`, `vendor`, `dist`, `build`, `coverage`, caches, generated output, worktrees). Ask before questionable excludes.

Run:

```bash
desloppify scan --path <path>
desloppify next
```

Loop until done or blocked:
1. Fix the current `next` item.
2. Run the smallest relevant check.
3. Run its resolve command.
4. Run `desloppify next`.

Use `backlog`, `plan`, or `plan queue` only when needed. Do not suppress, over-exclude, delete tests, or make cosmetic-only changes. Rescan before finishing when practical.

Final response: path, exclusions, changed files, checks, final Desloppify state, blockers.
