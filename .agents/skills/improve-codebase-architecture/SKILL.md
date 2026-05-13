---
name: improve-codebase-architecture
description: Find high-leverage architecture improvements. Use when the user wants refactoring opportunities, deeper modules, better seams, less coupling, clearer ownership, stronger domain boundaries, AI-navigability, or a more testable codebase.
register_cmd: true
---

# Improve Codebase Architecture

Do not hunt architecture improvements alone. Use fresh-context subagents because architecture review is biased by session framing; your job is to ground the scope, dispatch role-specific scouts, validate candidates, and return only small high-leverage design moves. Do not edit product code.

1. **Resolve scope.** Use `scope`, `$ARGUMENTS`, the conversation target, or the current repo. Prefer the smallest coherent surface with callers, tests, and future change pressure. If that cannot be bounded quickly, ask one narrowing question.

2. **Build the context packet.** Read relevant `CONTEXT.md`, `CONTEXT-MAP.md`, `docs/adr/**`, `AGENTS.md`, `CLAUDE.md`, `REVIEW.md`, README, entry points, key interfaces, representative callers, and nearby tests. Paste focused excerpts, not whole unrelated files. Include domain vocabulary and ADR constraints so scouts do not invent names or re-litigate decisions.

3. **Shared vocabulary.** A module has an interface and implementation. Interface means everything callers must know: types, invariants, errors, ordering, config, lifecycle. Depth means much behavior behind a small interface. A seam lets behavior vary without editing callers. Locality means one behavior change touches few places. Leverage means callers get more capability than the interface costs. Use the deletion test: deleting a shallow module makes complexity vanish; deleting a deep module spreads complexity across callers.

4. **Dispatch four scouts in parallel.** Each scout gets only `{SCOPE}`, `{CODE_CONTEXT}`, `{CONVENTIONS}`, and its role contract below. Require output as `Files`, `Friction`, `Future change path`, `Move`, `Payoff`, `Trade-off`. One candidate = one root cause. Skip style, renames, generic layering, speculative rewrites, praise, and "consider also".

   - **Module Depth Scout**
     - Mission: find shallow modules and deepen them.
     - Inspect: pass-through APIs, interfaces nearly as complex as implementations, repeated caller knowledge, modules that fail the deletion test.
     - Reject: extracting helpers, adding interfaces, or moving files unless callers gain locality/leverage.
     - Prefer moves that hide ordering, config, errors, lifecycle, or policy behind a smaller interface.

   - **Change Locality Scout**
     - Mission: find places where one behavior change requires many edits.
     - Inspect: duplicated orchestration, dependency direction, caller-side branching, leaked decisions, shotgun change paths, cross-module knowledge.
     - Reject: "reduce coupling" claims without a concrete future change path.
     - Prefer moves that concentrate a future change in one module or seam.

   - **Domain Invariant Scout**
     - Mission: find missing concepts and weak ownership.
     - Inspect: collapsed domain terms, invalid states, repeated policy, lifecycle ambiguity, data ownership confusion, ADR/context drift.
     - Reject: new nouns not grounded in repo language or user domain language.
     - Prefer moves that name a real concept and make invalid states or policy duplication harder.

   - **Seam and Test Surface Scout**
     - Mission: find seams that make behavior easier to test and vary.
     - Inspect: IO/time/network/global state, over-mocking, test-only pure functions with no locality, fake seams with one adapter, missing public test surfaces.
     - Reject: seams with no plausible second adapter or tests that only assert implementation detail.
     - Prefer moves where the interface becomes the test surface.

5. **Cluster and filter.** Wait for all scouts. Dedupe by root cause. Drop candidates without file evidence, without a realistic future change path, or whose payoff is aesthetic. Validate at most the six strongest.

6. **Validate with fresh-context subagents.** One validator per candidate, in parallel when possible. Prompt each validator:

   > Candidate: {CANDIDATE}. Relevant files/artifacts in full or focused excerpts: {FILES_OR_EXCERPTS}. Conventions and domain vocabulary: {CONVENTIONS}. Confirm or refute. State concrete evidence: current friction, future change path, why the current module/interface/seam/invariant is costly, smallest safe design move, locality/leverage/testability payoff, and accepted trade-off. Score 0-100. Score >=70 requires file-backed evidence, a concrete future change path, and a move smaller than a rewrite.

   Drop every candidate below 70.

7. **Report validated moves only.** Rank by confidence, payoff, and smallness. For each survivor include: `Files`, `Friction`, `Future change path`, `Move`, `Payoff`, `Trade-off`, `Confidence`. If none survive, say no high-leverage architecture improvement was found in the reviewed scope. Stop after candidates and ask which one to investigate, plan, or implement. Ask before updating `CONTEXT.md` or ADRs.

Think like a long-term steward: deep modules, hidden decisions, clear ownership, preserved invariants, fewer edit sites. Never invent files, line numbers, owners, milestones, or risks. Never dispatch this skill recursively; scouts and validators are one-shot subagents using the role contracts above.
