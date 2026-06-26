---
name: resume-branch-work
description: Resume and finish in-progress branch/worktree changes from the cumulative diff, optional notes, or plan, then verify done-done.
---

# Resume Branch Work

## Validation reuse and check scope

Before running a slow, broad, external, stateful, or CI-equivalent command, check whether this conversation or current-SHA CI/artifacts already contain usable proof. Reuse prior passing evidence instead of rerunning only when it is visible, ran after the last relevant edit, covers the same command/scenario and behavior, edge case, or public boundary, and no touched file, config, dependency, fixture, generated output, runtime state, or environment assumption it depends on changed afterward. If uncertain, run the narrowest freshness check that resolves the uncertainty before escalating.

Default to the narrowest honest proof. Run broader suites, full builds, CI reruns, or live/E2E flows only when required by blast radius, merge/release policy, changed shared API/schema/build/test infrastructure/dependencies/auth/security/persistence/concurrency, merge/conflict integration risk, missing targeted seams, or because the broad command is the only proof that covers the behavior.

Final reports must distinguish reused proof, newly run commands, and checks intentionally not run.

Act as a fresh agent inheriting an in-progress branch or dirty worktree. Do not rely on session memory. Use `context`, `plan`, `$ARGUMENTS`, or pasted notes only to clarify intent that is supported by the effective diff; the cumulative branch/worktree change set is the source of truth.

## 1. Resolve evidence before editing

Inspect and record the basics:
- repo root, current branch, `git status --short`, and upstream/ahead state when available
- local instructions governing changed or untracked paths, including relevant `AGENTS.md`, `CLAUDE.md`, README, CONTRIBUTING, and nearby docs
- comparison ref for committed branch changes: resolve the remote default branch from `origin/HEAD`; fall back to `origin/main` or `origin/master` only if `origin/HEAD` cannot be resolved. When a ref is available, set `base` with `git merge-base <comparison-ref> HEAD`.

Build one effective change set from every available source:
- committed branch changes: `git log base..HEAD --oneline` and `git diff base..HEAD` when `base` exists
- staged changes: `git diff --cached`
- unstaged changes: `git diff`
- untracked files: `git ls-files --others --exclude-standard`, rendered as new-file diffs or concise summaries for large/binary/generated files

Always include staged, unstaged, and untracked changes, even when committed branch changes exist. If the effective diff is empty and no concrete context or plan was provided, stop with a one-line note that there is nothing to resume. If provided context conflicts with the diff in a way that changes the goal, ask one precise question and stop.

## 2. Reconstruct the work

Before changing code, briefly state:
- `Branch intent:` the highest-level behavior, artifact, or system change the effective diff is trying to deliver
- `Already done:` what the diff proves is implemented, wired, documented, or tested
- `Remaining work:` the smallest incomplete, risky, or unverified pieces needed for done-done
- `Acceptance signal:` the command, test, observable behavior, or artifact check that will prove completion
- `Canonical patterns:` sibling files, tests, configs, or docs that show the repo's normal way to solve this kind of work

Validate the model against real repo paths. Read changed entry points, state/model code, callers, public contracts, config/doc consumers, tests, and two or three sibling implementations as risk warrants. Search may locate surfaces, but it does not prove behavior.

For non-trivial work, maintain an explicit todo/plan. Each item must be an observable behavior, integration point, verification gap, or acceptance condition. Keep exactly one item in progress and update it as work advances.

## 3. Finish the branch

If the branch already appears implementation-complete, do not invent work. Move to verification, self-review, and any narrow cleanup needed to make the final diff intentional.

Otherwise, implement the smallest coherent slice that makes the branch complete through real repo paths. Include production code, wiring, tests, fixtures, docs, config, migrations, or generated artifacts only when required for reachable behavior or a verifiable artifact contract.

Fix root causes. Preserve existing user changes. Match local style. Do not add TODO placeholders, suppressions, skipped or weakened tests, fake fixtures, broad dependency bumps, unrelated refactors, or formatter-only churn unless the branch goal explicitly requires them.

## 4. Done-done gate

Before claiming completion:
1. Re-read the provided context or plan and the final effective diff. Account for every requirement as implemented, already satisfied, intentionally out of scope, or blocked.
2. Confirm the changed behavior or artifact is reachable through the intended public entry point, command, UI, API, worker, config consumer, or documented path.
3. Ensure tests or equivalent checks would fail for at least one realistic regression in the branch goal. Add or strengthen targeted checks when a suitable seam exists and behavior would otherwise be unproven.
4. Validate through the reuse/scope policy: reuse fresh prior proof when valid, otherwise run targeted checks first, then broader relevant checks only when justified. Fix failures caused by this work. Identify unrelated failures without chasing them.
5. Inspect final status and diff for accidental files, secrets, debug output, generated noise, and unrelated edits.

No `complete`, `done`, `fixed`, `ready`, or `passing` language without fresh evidence from this conversation or current-SHA artifacts.

## Final response

Keep the final under 30 lines:

```text
Branch intent: <one sentence grounded in the effective diff>
Completed:
- <behavior/artifact>: <files> — <proof or check>
Checks:
- `<command>` -> <result> | `reused — <prior proof and why still valid>` | `not run — <reason>`
Deferred/blocked:
- <item or None>: <reason>
Final status: <clean or remaining modified/untracked files>
Next: <publish/review command or exact next action>
```
