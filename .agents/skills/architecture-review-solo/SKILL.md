---
name: architecture-review-solo
description: Direct codebase architecture review without subagents. Use only when the user explicitly asks for direct, inline, single-agent, or no-subagent architecture review.
register_cmd: true
---

# Architecture Review Solo

Review codebase architecture directly in this session. Do not invoke subagents. Preserve the same quality bar as `architecture-review` by doing explicit, sequential architecture-review passes and validating every candidate against concrete files, current friction, and a realistic future change path. Do not edit product code.

1. **Resolve scope.** Use `scope`, `$ARGUMENTS`, the conversation target, or the current repo. Prefer the smallest coherent surface with callers, tests, and future change pressure. If that cannot be bounded quickly after reading available context, ask one narrowing question.

2. **Build the context packet for yourself.** Read relevant `CONTEXT.md`, `CONTEXT-MAP.md`, `docs/adr/**`, `AGENTS.md`, `CLAUDE.md`, `REVIEW.md`, README, entry points, key interfaces, representative callers, and nearby tests. Keep excerpts focused. Capture domain vocabulary and ADR constraints so you do not invent names or re-litigate decisions.

3. **Shared vocabulary.** A module has an interface and implementation. Interface means everything callers must know: types, invariants, errors, ordering, config, lifecycle. Depth means much behavior behind a small interface. A seam lets behavior vary without editing callers. Locality means one behavior change touches few places. Leverage means callers get more capability than the interface costs. Use the deletion test: deleting a shallow module makes complexity vanish; deleting a deep module spreads complexity across callers.

4. **Run four direct architecture passes.** Do not report during the passes. Collect candidates only when they have file-backed evidence, current friction, a concrete future change path, a small safe move, payoff, and trade-off. One candidate = one root cause. Skip style, renames, generic layering, speculative rewrites, praise, and "consider also".

   - **Module Depth Pass**
     - Mission: find shallow modules and deepen them.
     - Inspect: pass-through APIs, interfaces nearly as complex as implementations, repeated caller knowledge, modules that fail the deletion test.
     - Reject: extracting helpers, adding interfaces, or moving files unless callers gain locality/leverage.
     - Prefer moves that hide ordering, config, errors, lifecycle, or policy behind a smaller interface.

   - **Change Locality Pass**
     - Mission: find places where one behavior change requires many edits.
     - Inspect: duplicated orchestration, dependency direction, caller-side branching, leaked decisions, shotgun change paths, cross-module knowledge.
     - Reject: "reduce coupling" claims without a concrete future change path.
     - Prefer moves that concentrate a future change in one module or seam.

   - **Domain Invariant Pass**
     - Mission: find missing concepts and weak ownership.
     - Inspect: collapsed domain terms, invalid states, repeated policy, lifecycle ambiguity, data ownership confusion, ADR/context drift.
     - Reject: new nouns not grounded in repo language or user domain language.
     - Prefer moves that name a real concept and make invalid states or policy duplication harder.

   - **Seam and Test Surface Pass**
     - Mission: find seams that make behavior easier to test and vary.
     - Inspect: IO/time/network/global state, over-mocking, test-only pure functions with no locality, fake seams with one adapter, missing public test surfaces.
     - Reject: seams with no plausible second adapter or tests that only assert implementation detail.
     - Prefer moves where the interface becomes the test surface.

5. **Cluster, filter, and validate yourself.** Dedupe by root cause. Drop candidates without file evidence, without a realistic future change path, or whose payoff is aesthetic. Validate at most the six strongest by re-reading the relevant files/artifacts in full or focused excerpts. Keep only candidates that can show concrete evidence: current friction, future change path, why the current module/interface/seam/invariant is costly, smallest safe design move, locality/leverage/testability payoff, and accepted trade-off.

6. **Report validated moves only.** Rank by confidence, payoff, and smallness. For each survivor include: `Files`, `Friction`, `Future change path`, `Move`, `Payoff`, `Trade-off`, `Confidence`. If none survive, say no high-leverage architecture improvement was found in the reviewed scope. Stop after candidates and ask which one to investigate, plan, or implement. Ask before updating `CONTEXT.md` or ADRs.

Think like a long-term steward: deep modules, hidden decisions, clear ownership, preserved invariants, fewer edit sites. Never invent files, line numbers, owners, milestones, or risks. Do not use subagents.
