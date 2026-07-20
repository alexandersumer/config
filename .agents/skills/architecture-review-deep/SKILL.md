---
name: architecture-review-deep
description: Heavyweight codebase architecture review using fresh-context Reviewer candidate generation and direct validation by this agent. Use when the user asks for deep, thorough, multi-agent, high-confidence, or heavyweight review of architecture, modularity, seams, coupling, ownership, domain boundaries, AI-navigability, or testability. Use architecture-review-solo for ordinary/direct single-agent architecture review.
---

# Architecture Review Deep

This is the heavyweight architecture-review path. Use fresh-context Reviewers for candidate generation because architecture review is biased by session framing. Use the current harness's native managed subagent mechanism. If no native managed mechanism can provide separate fresh contexts and collect a terminal result from each, stop with `Review inconclusive` and name the missing capability. Do not simulate Reviewers with external agent CLIs or unmanaged processes. Reviewer output is candidate evidence, not authority. This agent owns validation and final architectural judgment. Do not edit product code.

1. **Resolve scope.** Use `scope`, `$ARGUMENTS`, the conversation target, or the current repo. Prefer the smallest coherent surface with callers, tests, and future change pressure. If that cannot be bounded quickly, ask one narrowing question.

2. **Build the context packet.** Read relevant `CONTEXT.md`, `CONTEXT-MAP.md`, `docs/adr/**`, `AGENTS.md`, `CLAUDE.md`, `REVIEW.md`, README, entry points, key interfaces, representative callers, error/logging paths, and nearby tests. Paste focused excerpts, not whole unrelated files. Include domain vocabulary and ADR constraints so Reviewers do not invent names or re-litigate decisions.

3. **Shared vocabulary.** A module has an interface and implementation. Interface means everything callers must know: types, invariants, errors, ordering, config, lifecycle. Depth means much behavior behind a small interface. A seam lets behavior vary without editing callers. Locality means one behavior change touches few places. Leverage means callers get more capability than the interface costs. Use the deletion test: deleting a shallow module makes complexity vanish; deleting a deep module spreads complexity across callers.

4. **Invoke four fresh-context architecture Reviewer passes through native managed subagents.** Create one separately prompted context per role. Give each Reviewer only the explicitly constructed evidence packet: `{SCOPE}`, `{CODE_CONTEXT}`, `{CONVENTIONS}`, and its role contract below; do not pass inherited session context, prior Reviewer output, or conclusions from this session. Run roles concurrently when the harness safely supports it. Otherwise run them sequentially or in bounded waves as separate fresh contexts; limited concurrency changes latency, not the review contract. Collect every initial result before validating output or candidates. Do not use external agent CLIs, unmanaged wrappers or processes, detached or scheduled execution, or recursive validation delegation. Require output as `Files`, `Friction`, `Future change path`, `Move`, `Payoff`, `Trade-off`. One candidate = one root cause. Skip style, renames, generic layering, speculative rewrites, praise, and "consider also".

   - **Module Depth Reviewer**
     - Mission: find shallow modules and deepen them.
     - Inspect: pass-through APIs, interfaces nearly as complex as implementations, repeated caller knowledge, modules that fail the deletion test.
     - Reject: extracting helpers, adding interfaces, or moving files unless callers gain locality/leverage.
     - Prefer moves that hide ordering, config, errors, lifecycle, or policy behind a smaller interface.

   - **Change Locality Reviewer**
     - Mission: find places where one behavior change requires many edits.
     - Inspect: duplicated orchestration, dependency direction, caller-side branching, leaked decisions, shotgun change paths, cross-module knowledge.
     - Reject: "reduce coupling" claims without a concrete future change path.
     - Prefer moves that concentrate a future change in one module or seam.

   - **Domain Invariant Reviewer**
     - Mission: find missing concepts and weak ownership.
     - Inspect: collapsed domain terms, invalid states, repeated policy, lifecycle ambiguity, data ownership confusion, ADR/context drift.
     - Reject: new nouns not grounded in repo language or user domain language.
     - Prefer moves that name a real concept and make invalid states or policy duplication harder.

   - **Seam and Test Surface Reviewer**
     - Mission: find seams that make behavior easier to test and vary.
     - Inspect: IO/time/network/global state, diagnostics/logging emitted far from the owning concept, noisy cross-cutting logs, unclear error surfaces, over-mocking, test-only pure functions with no locality, fake seams with one adapter, missing public test surfaces.
     - Reject: seams with no plausible second adapter or tests that only assert implementation detail.
     - Prefer moves where the interface becomes the test surface.

5. **Validate Reviewer output shape, then cluster and filter.** A Reviewer response is valid only if it uses the requested candidate fields or clearly says no findings for its role. Empty, whitespace-only, truncated, unavailable, timed out, or otherwise unstructured output is invalid. After collecting all initial results, retry only invalid roles once through the same native managed mechanism with a smaller evidence packet and a new fresh context. Run multiple retries concurrently when safely supported; otherwise run them separately. Do not rerun valid roles. If any retried role is still invalid, stop with `Review inconclusive`. Never fall back to external agent CLIs or unmanaged processes. Then dedupe by root cause. Drop candidates without file evidence, without a realistic future change path or regression-investigation path, or whose payoff is aesthetic. Validate at most the six strongest directly in this session.

6. **Validate candidate moves directly in this session.** Do not delegate validation of Reviewer findings to another subagent, validator, or recursive review chain. For each candidate, inspect the relevant files/artifacts in full or focused excerpts yourself. Confirm or refute with concrete evidence: current file-backed friction, concrete future change path or regression-investigation path, why the current module/interface/invariant is costly, smallest safe move, locality/leverage/debuggability/testability payoff, and accepted trade-off. Drop anything below score-equivalent confidence of 70; score-equivalent confidence of at least 70 requires file-backed evidence, a concrete future change path or regression-investigation path, and a move smaller than a rewrite. Unvalidated candidates are never reported as findings; if required evidence cannot be obtained after focused inspection, report no validated move or an explicit inconclusive evidence gap.

7. **Report validated moves only.** Rank by confidence, payoff, and smallness. For each survivor include: `Files`, `Friction`, `Future change path`, `Move`, `Payoff`, `Trade-off`, `Confidence`. If none survive, say no high-leverage architecture improvement was found in the reviewed scope. Stop after candidates and ask which one to investigate, plan, or implement. Ask before updating `CONTEXT.md` or ADRs.

Think like a long-term steward: deep modules, hidden decisions, clear ownership, preserved invariants, fewer edit sites. Never invent files, line numbers, owners, milestones, or risks. Never dispatch this skill recursively; use only native managed reviewer contexts, let Reviewers generate candidates only, and validate directly in this session.
