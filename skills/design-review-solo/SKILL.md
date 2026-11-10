---
name: design-review-solo
description: Direct design review without subagents. Use for ordinary, direct, inline, single-agent, or no-subagent review of software designs, plans, architecture proposals, API boundaries, module splits, or data models. Use design-review-deep when the user asks for deep, thorough, multi-agent, high-confidence, or heavyweight design review, and review-solo for code changes.
---

# Design Review Solo

Review the design directly in this session. Do not invoke subagents. Preserve the same quality bar as `design-review-deep` by grounding the review in the artifact, diff, code context, conventions, and concrete future change paths.

1. **Resolve the scope.** Use `$ARGUMENTS`, the conversation, or a named artifact or file; otherwise default to the effective branch/worktree diff, which needs no PR, explicit scope, or committed changes. Treat an explicit artifact, file, directory, module, symbol, subsystem, or API as a standalone target. For a diff, compare against the requested base or PR destination, else a valid `origin/HEAD`, else the sole existing `origin/main` or `origin/master`, and set `<base>` to `git merge-base <comparison-ref> HEAD`. If no unambiguous base exists, say so, use `HEAD`, and review the working-tree portion. Treat `git diff <base>` plus untracked files from `git ls-files --others --exclude-standard` as one final diff; read the committed, staged, and unstaged component diffs only for context, because changes that cancel out are not part of the result. For an explicit staged-only or commit-specific request, use that diff instead. Use `$ARGUMENTS` only to narrow it. If no artifact, file, or diff can be found and the target is still unclear, ask one precise question and stop.

2. **Ground the review in real context.** Read the design/artifact/diff plus enough surrounding code to name the domain vocabulary, ownership boundaries, callers, data flow, invariants, persistence/API seams, rollout path, tests, and local conventions. Find every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` whose directory is relevant to the scoped files.

3. **Run four direct design passes.** Do not report during the passes. Collect candidate issues only when grounded in code, artifact text, or a concrete future change path:
   - **Boundaries and ownership** — wrong module/API seams, leaked decisions, change amplification, misplaced responsibility.
   - **Domain and invariants** — collapsed concepts, broken aggregate/data invariants, weak ubiquitous language, invalid state transitions.
   - **Evolution and operability** — migration/rollout/rollback risk, observability gaps, unclear diagnostic/error model, low-signal or noisy logging strategy, concurrency/idempotency, persistence/API compatibility, future change cost.
   - **Simplicity and abstraction** — shallow wrappers, pass-through APIs, generic non-abstractions, configuration sprawl, temporal decomposition, unnecessary layers.

4. **Validate every candidate yourself.** Confirm or refute each issue with concrete evidence: boundary crossed, invariant weakened, compatibility risk introduced, change path made expensive, regression investigation made materially harder by poor errors/logging, operational gap created, or convention violated. Drop anything speculative, taste-based, generic, or not material to the scoped design. Do not keep a finding merely because it sounds architecturally sophisticated.

5. **Report.** Dedupe by root cause and rank Critical, High, Medium, Low. For each surviving issue, include severity, location/artifact section if available, architectural judgment, evidence, design move, and accepted trade-off — one concise paragraph each. If the cheapest good design is no change, say so. If the review surface is too large to validate directly, say `Review limited` and name the residual risk. If no material issue remains, say in one line that no material design issue was found and name the review surface.

Think like a long-term codebase steward, not a style reviewer. Prefer designs that reduce total complexity: deep modules, hidden decisions, clear ownership, preserved invariants, and fewer places to edit for one behavior change. Never invent files, line numbers, owners, milestones, or risks. Never approve or merge. Do not use subagents.
