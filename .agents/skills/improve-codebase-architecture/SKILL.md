---
name: improve-codebase-architecture
description: Find high-leverage architecture improvements. Use when the user wants refactoring opportunities, deeper modules, better seams, less coupling, or a more testable codebase.
register_cmd: true
---

Find high-leverage architecture improvements in `scope`, `$ARGUMENTS`, the conversation target, or the current repo. Do not edit product code. Discover, validate, and rank deepening opportunities; implementation happens only after the user chooses one.

You are the orchestrator, not the sole architect. Ground the scope yourself, then use fresh-context subagents as independent scouts and validators. Subagents have no session context; all facts must be pasted into their prompts.

## Grounding

Read domain and decision context first when present: `CONTEXT.md`, `CONTEXT-MAP.md`, relevant `docs/adr/**`, `AGENTS.md`, `CLAUDE.md`, `REVIEW.md`, README, and nearby tests. Use the repo's domain words, not invented architecture labels.

Shared vocabulary to include in scout and validator prompts:
- Module: code with an interface and implementation.
- Interface: everything callers must know: types, invariants, errors, ordering, config, lifecycle.
- Depth: much behavior behind a small interface.
- Seam: where behavior can vary without editing callers.
- Adapter: concrete implementation at a seam.
- Locality: one behavior change touches few places.
- Leverage: callers get more capability than the interface cost.

Look for real friction: shallow pass-through modules, leaked invariants, repeated caller knowledge, hard-to-test behavior, shotgun changes, fake seams, or ADRs that no longer match reality. Use the deletion test: if deleting a module makes complexity disappear, it was probably shallow; if deleting it spreads complexity across callers, it was earning depth.

## Process

1. **Resolve the scope.** Prefer explicit `scope` or `$ARGUMENTS`; otherwise use the conversation target; otherwise inspect the repo's entry points and pick the smallest coherent area with identifiable callers and tests. If no meaningful scope can be bounded, ask one clarifying question instead of reviewing the whole repo blindly.

2. **Build one shared context packet.** Include only evidence scouts need: module/file map, key interfaces, representative call paths, focused excerpts over full-file dumps, relevant test names/assertions, conventions, ADR/context excerpts, and any concrete future change the user cares about. Exclude generated files, lockfiles, and unrelated implementation detail.

3. **Dispatch four fresh-context architecture scouts in parallel.** Each scout gets this prompt verbatim, with `{ROLE}`, `{SCOPE}`, `{CODE_CONTEXT}`, and `{CONVENTIONS_AND_VOCABULARY}` filled in:

   > You are scouting architecture improvements as **{ROLE}**. Scope: {SCOPE}. Code/context: {CODE_CONTEXT}. Conventions, decisions, and vocabulary: {CONVENTIONS_AND_VOCABULARY}. Return candidates using this exact shape: `Files`, `Friction`, `Future change path`, `Move`, `Payoff`, `Trade-off`. Only include candidates backed by files and a concrete future change path. Skip style, renames, generic layering, speculative rewrites, praise, and "consider also" advice. If no material candidate exists, say so.

   Roles:
   - **Module depth and interface leverage** — shallow wrappers, pass-through modules, interfaces that expose too much, modules that should absorb repeated caller knowledge.
   - **Coupling and locality** — shotgun changes, leaked decisions, dependency direction problems, feature logic scattered across unrelated files.
   - **Domain concepts and invariants** — collapsed concepts, repeated policy, invalid states, ownership confusion, ADR/context drift from code reality.
   - **Testability and seams** — hard-to-test IO/time/network/global behavior, fake seams with one adapter, over-mocking caused by missing public seams.

4. **Cluster before validating.** Wait for all scouts. Dedupe candidates by root cause, discard weakly evidenced duplicates, and validate only the best-evidenced candidate per root cause. If there are many, validate the highest-leverage six at most.

5. **Validate candidates in parallel.** Use one fresh Task subagent per candidate. Each validator gets this prompt verbatim, with `{CANDIDATE}`, `{FILES_OR_EXCERPTS}`, and `{CONVENTIONS_AND_VOCABULARY}` filled in:

   > Candidate: {CANDIDATE}. Relevant files/artifacts in full or focused excerpts: {FILES_OR_EXCERPTS}. Conventions, decisions, and vocabulary: {CONVENTIONS_AND_VOCABULARY}. Confirm or refute. State concrete evidence: current friction, future change path, why the existing interface/coupling/invariant/seam is costly, smallest safe design move, locality/leverage/testability payoff, and accepted trade-off. Score 0–100. Score >=80 requires file-backed evidence, demonstrated leverage, a concrete future change path, and a move smaller than a rewrite. Score <80 if any are absent or weak.

   Drop every candidate below 80. Do not keep a finding because it sounds architecturally sophisticated.

6. **Report only validated candidates.** Dedupe by root cause again and rank by validator confidence, then by payoff, then by smallest safe move. Stop after candidates and ask the user which one to investigate, plan, or implement.

For each surviving candidate include:
- Files: `<paths>`
- Friction: `<what is costly now>`
- Move: `<specific design move>`
- Payoff: `<locality/leverage/testability improvement>`
- Trade-off: `<accepted cost>`
- Confidence: `<validator score and why>`

If no candidate survives, say no high-leverage architecture improvement was found in the reviewed scope and name the scope. If a decision should update `CONTEXT.md` or an ADR, ask explicitly before writing docs.

Never propose generic cleanups, style, renames, layering theater, or new abstractions without leverage. Never invent files, line numbers, owners, milestones, or risks. Never dispatch this skill recursively; scouts and validators are one-shot Task subagents using the prompts above.
