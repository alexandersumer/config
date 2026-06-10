---
name: design-review-deep
description: Heavyweight design review using fresh-context Reviewer candidate generation and direct Axiom validation. Use when the user asks for deep, thorough, multi-agent, high-confidence, or heavyweight review of a software design, architecture proposal, API boundary, module split, data model, or design-sensitive diff. Use design-review-solo for ordinary/direct single-agent design review.
---

# Design Review Deep

This is the heavyweight design-review path. You do not rely on the author's framing alone. Fresh-context Reviewers challenge the design from different angles and produce candidate evidence, not authority. Axiom owns validation and final architectural judgment.

1. **Resolve the scope and effective diff automatically.** Use `scope`, `$ARGUMENTS`, the conversation, artifact, or file when provided; otherwise default to the effective branch/worktree diff. Do not require a PR, explicit scope, or committed branch changes. For diff review, resolve the remote default branch from `origin/HEAD`, falling back to `origin/main` or `origin/master` only if needed; if no remote default exists, omit only the committed-branch part. Build one effective diff from the union of: committed branch changes with `git diff $(git merge-base <remote-default> HEAD)..HEAD`, staged changes with `git diff --cached`, unstaged changes with `git diff`, and untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs. Always include staged, unstaged, and untracked changes even when the committed branch diff exists. If `scope` or `$ARGUMENTS` is provided, use it only to narrow this discovered diff unless it names a separate artifact or file. If no artifact/file/diff can be discovered and the design target is still unclear after reading available context, ask one precise question and stop.

2. **Ground the review yourself.** Read the design/artifact/diff plus enough surrounding code to name the real domain vocabulary, ownership boundaries, callers, data flow, invariants, persistence/API seams, rollout path, tests, and local conventions. Find every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` whose directory is relevant to the scoped files and include those conventions in reviewer prompts.

3. **Dispatch four fresh-context design Reviewer passes in parallel.** Before dispatch, confirm `{DESIGN}` and `{CODE_CONTEXT}` contain non-empty pasted artifact/code text or focused excerpts, not just paths, filenames, or summaries. Each Reviewer pass gets this prompt verbatim, with `{ROLE}`, `{DESIGN}`, `{CODE_CONTEXT}`, and `{CONVENTIONS}` filled in. No session context — only what you paste:

   > You are reviewing a software design as **{ROLE}**. Design/artifact/diff: {DESIGN}. Relevant code context: {CODE_CONTEXT}. Conventions: {CONVENTIONS}. One issue = one root cause. Ground every issue in code, artifact text, or a concrete future change path. Skip taste, style, generic best practices, praise, and "consider also" advice. Return exactly one of:
   >
   > CANDIDATES:
   > - severity: <Critical | High | Medium | Low>
   >   location: <file path, line, or artifact section>
   >   claim: <architectural issue>
   >   evidence: <boundary, invariant, compatibility, or change-path evidence>
   >   suggested_fix: <minimal design move>
   >
   > NO_FINDINGS
   > Reviewed: <files/scope>
   > Reason: <one sentence>

   Roles:
   - **Boundaries and ownership** — wrong module/API seams, leaked decisions, change amplification, misplaced responsibility
   - **Domain and invariants** — collapsed concepts, broken aggregate/data invariants, weak ubiquitous language, invalid state transitions
   - **Evolution and operability** — migration/rollout/rollback risk, observability gaps, concurrency/idempotency, persistence/API compatibility, future change cost
   - **Simplicity and abstraction** — shallow wrappers, pass-through APIs, generic non-abstractions, configuration sprawl, temporal decomposition, unnecessary layers

4. **Validate Reviewer output shape before candidate validation.** A Reviewer response is valid only if it contains either `CANDIDATES:` or `NO_FINDINGS`. Empty, whitespace-only, truncated, or otherwise unstructured output is invalid. If any Reviewer output is invalid, retry that Reviewer once with a smaller pasted design/code/context packet. If it is still invalid, stop with `Review inconclusive`. Never treat invalid output as no findings.

5. **Validate every candidate directly in Axiom.** Do not delegate validation of Reviewer findings to another subagent, validator, or recursive review chain. For each candidate, Axiom must inspect the relevant artifacts, code, callers, data flow, contracts, rollout path, tests, and conventions itself. Confirm or refute with concrete design evidence: boundary crossed, invariant weakened, compatibility risk introduced, change path made expensive, operational gap created, or convention violated. Drop anything Axiom cannot demonstrate concretely with score-equivalent confidence of at least 70. Do not keep a finding merely because it sounds architecturally sophisticated. Unvalidated candidates are never reported as findings; if required evidence cannot be obtained after focused inspection, mark the review inconclusive instead of forwarding the candidate.

6. **Report.** Dedupe by root cause and rank Critical, High, Medium, Low. For each surviving issue, include severity, location/artifact section if available, architectural judgment, evidence, design move, and accepted trade-off — one concise paragraph each. If any Reviewer output is invalid after retry or Axiom cannot obtain required validation evidence, end with `Review inconclusive` and the failed role or evidence gap. If the cheapest good design is no change, say so. If zero candidates survive direct validation, say in one line that no material design issue was found and name the review surface.

Think like a long-term codebase steward, not a style reviewer. Prefer designs that reduce total complexity: deep modules, hidden decisions, clear ownership, preserved invariants, and fewer places to edit for one behavior change. Never invent files, line numbers, owners, milestones, or risks. Never approve or merge.
