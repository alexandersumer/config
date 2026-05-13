---
name: design-review
description: Use when reviewing software design, architecture, a technical proposal, refactor plan, API boundary, module split, data model, or design-sensitive branch diff.
register_cmd: true
---

# Design Review

You do not rely on the author's framing alone. Fresh-context subagents challenge the design from different angles; your job is to provide the design artifact, relevant code context, conventions, and then aggregate only validated architectural findings.

1. **Resolve the scope.** Use `scope`, `$ARGUMENTS`, the conversation, artifact, file, or effective branch/worktree diff. When reviewing a diff, resolve the remote default branch from `origin/HEAD`, falling back to `origin/main` or `origin/master` only if needed, then review the union of committed branch changes, staged changes, unstaged changes, and untracked files: `git diff $(git merge-base <remote-default> HEAD)..HEAD`, `git diff --cached`, `git diff`, and `git ls-files --others --exclude-standard` rendered as new-file diffs. If the committed branch diff is empty but the working tree has staged, unstaged, or untracked changes, review those working-tree changes instead of stopping. If the design target is unclear after reading available context, ask one precise question and stop.

2. **Ground the review yourself.** Read the design/artifact/diff plus enough surrounding code to name the real domain vocabulary, ownership boundaries, callers, data flow, invariants, persistence/API seams, rollout path, tests, and local conventions. Find every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` whose directory is relevant to the scoped files and include those conventions in reviewer prompts.

3. **Dispatch four fresh-context design reviewers in parallel.** Each reviewer gets this prompt verbatim, with `{ROLE}`, `{DESIGN}`, `{CODE_CONTEXT}`, and `{CONVENTIONS}` filled in. No session context — only what you paste:

   > You are reviewing a software design as **{ROLE}**. Design/artifact/diff: {DESIGN}. Relevant code context: {CODE_CONTEXT}. Conventions: {CONVENTIONS}. Return candidate design issues or explicitly say no issue. One issue = one root cause. Ground every issue in code, artifact text, or a concrete future change path. Skip taste, style, generic best practices, praise, and "consider also" advice.

   Roles:
   - **Boundaries and ownership** — wrong module/API seams, leaked decisions, change amplification, misplaced responsibility
   - **Domain and invariants** — collapsed concepts, broken aggregate/data invariants, weak ubiquitous language, invalid state transitions
   - **Evolution and operability** — migration/rollout/rollback risk, observability gaps, concurrency/idempotency, persistence/API compatibility, future change cost
   - **Simplicity and abstraction** — shallow wrappers, pass-through APIs, generic non-abstractions, configuration sprawl, temporal decomposition, unnecessary layers

4. **Validate every candidate.** Use one fresh Task subagent per candidate issue. Each validator gets this prompt verbatim, with `{ISSUE}`, `{FILES_OR_ARTIFACTS}`, and `{CONVENTIONS}` filled in:

   > Issue: {ISSUE}. Relevant files/artifacts in full or as focused excerpts: {FILES_OR_ARTIFACTS}. Conventions: {CONVENTIONS}. Confirm or refute. State concrete evidence: boundary crossed, invariant weakened, change path made expensive, compatibility risk introduced, or convention violated. Score 0–100; anything you cannot demonstrate concretely scores under 80.

   Drop every candidate below 80. Do not keep a finding merely because it sounds architecturally sophisticated.

5. **Report.** Dedupe by root cause and rank Critical, High, Medium, Low. For each surviving issue, include severity, location/artifact section if available, architectural judgment, evidence, design move, and accepted trade-off — one concise paragraph each. If the cheapest good design is no change, say so. If zero candidates survive validation, say in one line that no material design issue was found and name the review surface.

Think like a long-term codebase steward, not a style reviewer. Prefer designs that reduce total complexity: deep modules, hidden decisions, clear ownership, preserved invariants, and fewer places to edit for one behavior change. Never invent files, line numbers, owners, milestones, or risks. Never approve or merge.
