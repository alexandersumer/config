---
name: desloppify
description: Run Desloppify's scan/next/resolve loop to improve code quality
argument-hint: "[optional: scan path]"
inputs:
  - name: path
    label: Scan path
    description: Directory to scan. Leave empty to scan the whole repository with `.`.
    type: string
    required: false
---

Run Desloppify on this repo and follow its queue. Assume `desloppify` is already installed; if it is missing, stop and say so.

Use `path` if provided; otherwise use `$ARGUMENTS`; otherwise use `.`.

Before scanning, inspect top-level directories. Exclude only obvious non-source directories such as `.git`, `node_modules`, `vendor`, `dist`, `build`, `coverage`, caches, generated output, or worktrees:

```bash
desloppify exclude <path>
```

Ask before excluding anything questionable.

Run:

```bash
desloppify scan --path <path>
desloppify next
```

Then loop:
1. Fix exactly the current `desloppify next` item.
2. Verify the fix with the smallest relevant repo check.
3. Run the resolve command Desloppify gave for that item.
4. Run `desloppify next` again.

Use `desloppify backlog` only to inspect broader work when the current queue is unclear. Use `desloppify plan` / `desloppify plan queue` only when reordering or grouping work would make the fixes cleaner.

Do not game the score with suppressions, unnecessary exclusions, deleted tests, or cosmetic churn. Fix root causes. Rescan periodically and before the final summary when practical.

Final response: scan path, exclusions, files changed, checks run, final Desloppify state, and anything blocked.
