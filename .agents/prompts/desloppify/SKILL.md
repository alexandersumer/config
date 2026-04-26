---
name: desloppify
description: Install and run Desloppify to improve code quality through its scan/next/resolve loop
argument-hint: "[optional: scan path or focus]"
inputs:
  - name: path
    label: Scan path
    description: Directory to scan with Desloppify. Leave empty to scan the whole repository with `.`.
    type: string
    required: false
  - name: focus
    label: Focus or instructions
    description: Optional scope, priority, or extra quality-improvement instructions.
    type: string
    required: false
---

<goal>
Improve the quality of this codebase by installing and running Desloppify, then following Desloppify's execution queue until the strict score is as high as practical for the session. Desloppify is the source of truth for what to fix next; do not replace its findings with unrelated personal cleanup ideas.
</goal>

<inputs>
Use `path` if provided; otherwise use `$ARGUMENTS` when it looks like a path; otherwise scan `.`. Treat `focus` and any remaining `$ARGUMENTS` as optional prioritization guidance, not as permission to skip Desloppify's workflow.
</inputs>

<setup>
1. Confirm Python 3.11+ is available. Prefer an existing `python3.11`, `python3.12`, or `python3.13` executable when the default `python3` is older.
2. Install Desloppify with the Python 3.11+ environment:
   - Direct environment: `pip install --upgrade "desloppify[full]"`
   - If the environment is externally managed, create a local virtualenv using Python 3.11+ and use its `pip`/`desloppify` commands.
3. Run the agent workflow install/update command adapted for the active agent. For Rovo, prefer the closest supported guide and state that it is being adapted locally:
   - `desloppify update-skill claude`
4. Do not create broad cleanup changes before the initial scan.
</setup>

<exclude_policy>
Before scanning, inspect top-level and near-top-level directories for obvious non-source content such as vendor dependencies, build output, generated code, caches, virtualenvs, package manager stores, coverage artifacts, worktrees, and VCS internals.

Exclude obvious candidates with `desloppify exclude <path>` before scanning. Good default examples include `.git`, local Desloppify/Python virtualenvs created only to run the tool, `node_modules`, `vendor`, `dist`, `build`, `coverage`, `.next`, `.turbo`, and generated output directories when clearly generated.

Do not exclude directories that contain repo-authored prompts, configuration, scripts, documentation, source, or tests just because they are small or unusual. Share questionable candidates with the user and wait for direction before excluding them.
</exclude_policy>

<scan>
Run:

```bash
desloppify scan --path <path>
desloppify next
```

The scan output includes agent instructions. Follow those instructions closely. If Desloppify gives a resolve command for an item, preserve it and run it only after the item is actually fixed.
</scan>

<loop>
The main job is the Desloppify loop:

1. Run `desloppify next`.
2. Read the currently queued work, target files, acceptance guidance, and resolve command.
3. Fix the queued item properly in production code, tests, docs, or configuration as appropriate.
4. Use the smallest meaningful verification command for the changed area. Add or strengthen tests when the quality issue is behavioral and tests are missing.
5. Run the exact Desloppify resolve command for the completed item.
6. Run `desloppify next` again.
7. Repeat until no queued work remains, the remaining work is blocked, or the session budget is exhausted.

Use `desloppify backlog` only when the current queue is insufficient to understand broader open work. Use `desloppify plan` and `desloppify plan queue` to reorder or cluster related issues when that improves the quality and coherence of the work.

Rescan periodically, especially after larger refactors or several resolved items:

```bash
desloppify scan --path <path>
desloppify next
```
</loop>

<quality_bar>
Fix root causes, not symptoms. Large refactors and small detailed fixes both matter. Do not game the score with suppressions, exclusions, superficial rewrites, deleted tests, or cosmetic-only churn. Preserve existing behavior unless Desloppify identifies behavior as the quality problem. Keep the working tree reviewable and explain any deferred or blocked items.
</quality_bar>

<verification_and_output>
Before finishing, run a final `desloppify scan --path <path>` when practical, then `desloppify next` to show the final queue state. Also run relevant repo-local tests/checks for touched files when available.

Final response should include:
- Scan path and exclusions used.
- Desloppify commands run and final queue/score summary.
- Files changed and the quality improvements made.
- Verification commands and results.
- Blocked/deferred items, if any.
- Suggested next action, usually continuing the Desloppify loop, reviewing the branch, or publishing changes.
</verification_and_output>
