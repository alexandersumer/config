---
name: resume-branch-work
description: Resume and finish in-progress branch/worktree changes from the cumulative diff, optional notes, or plan, then verify done-done.
---

# Resume Branch Work

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Act as a fresh agent inheriting an in-progress branch or dirty worktree; do not rely on session memory. Use explicit user context, `$ARGUMENTS`, a plan, or pasted notes as the source of intended scope; use the cumulative branch/worktree change set as the source of truth for current implementation state. Infer intent from the diff only when no stronger intent artifact exists.

## 1. Resolve evidence before editing

Record the repo root, current branch, `git status --short`, upstream/ahead state when available, and local instructions (`AGENTS.md`, `CLAUDE.md`, README, CONTRIBUTING) governing changed or untracked paths. Resolve the comparison ref from `origin/HEAD`, falling back to `origin/main` or `origin/master` only if `origin/HEAD` cannot be resolved, and set `<base>` with `git merge-base <comparison-ref> HEAD`.

Build one effective change set from every available source, always including staged, unstaged, and untracked changes even when committed branch changes exist:
- committed: `git log <base>..HEAD --oneline` and `git diff <base>..HEAD` when `<base>` exists
- staged: `git diff --cached`
- unstaged: `git diff`
- untracked: `git ls-files --others --exclude-standard`, rendered as new-file diffs or concise summaries for large/binary/generated files

If the effective diff is empty and no concrete context or plan was provided, stop with a one-line note that there is nothing to resume. If provided context conflicts with the diff in a way that changes the goal, ask one precise question and stop.

## 2. Reconstruct the work

Before changing code, briefly state:
- `Branch intent:` the highest-level intended change, grounded in the explicit intent artifact when provided and reconciled with the effective diff
- `Already done:` what the diff proves is implemented, wired, documented, or tested
- `Remaining work:` the smallest incomplete, risky, or unverified pieces needed for done-done
- `Acceptance signal:` the command, test, observable behavior, or artifact check that will prove completion
- `Canonical patterns:` sibling files, tests, configs, or docs that show the repo's normal approach

Validate this model against the real code paths; search locates surfaces but does not prove behavior. For non-trivial work, keep a todo list of observable behaviors, integration points, verification gaps, or acceptance conditions.

## 3. Finish the branch

If the branch already appears implementation-complete, do not invent work; move to verification, self-review, and any narrow cleanup needed to make the final diff intentional.

Otherwise, implement the smallest coherent slice that makes the branch complete through real repo paths, adding tests, docs, config, migrations, or generated artifacts only when required for reachable behavior or a verifiable artifact contract. Preserve existing user changes. Do not add TODO placeholders, suppressions, skipped or weakened tests, fake fixtures, broad dependency bumps, or formatter-only churn unless the branch goal explicitly requires them.

## 4. Done-done gate

Before claiming completion:
1. Re-read the provided context or plan and the final effective diff. Account for every requirement as implemented, already satisfied, intentionally out of scope, or blocked.
2. Confirm the changed behavior or artifact is reachable through the intended public entry point, command, UI, API, worker, config consumer, or documented path.
3. Ensure tests or equivalent checks would fail for at least one realistic regression in the branch goal; add or strengthen targeted checks when behavior would otherwise be unproven.
4. Validate under the proof policy. Fix failures caused by this work; identify unrelated failures without chasing them.
5. Inspect final status and diff for accidental files, secrets, debug output, generated noise, and unrelated edits.

No `complete`, `done`, `fixed`, `ready`, or `passing` language without fresh or validly reused proof.

## Final response

Keep the final under 30 lines:

```text
Branch intent: <one sentence grounded in the intended scope and final effective diff>
Completed:
- <behavior/artifact>: <files> — <proof or check>
Checks:
- `<command>` -> <result> | `reused — <prior proof and why still valid>` | `not run — <reason>`
Deferred/blocked:
- <item or None>: <reason>
Final status: <clean or remaining modified/untracked files>
Next: <publish/review command or exact next action>
```
