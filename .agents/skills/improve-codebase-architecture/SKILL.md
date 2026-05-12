---
name: improve-codebase-architecture
description: Find high-leverage architecture improvements. Use when the user wants refactoring opportunities, deeper modules, better seams, less coupling, or a more testable codebase.
---

Find deepening opportunities in `scope`, `$ARGUMENTS`, the current codebase, or the conversation target. Do not edit product code.

Read domain and decision context first when present: `CONTEXT.md`, `CONTEXT-MAP.md`, relevant `docs/adr/**`, `AGENTS.md`, `CLAUDE.md`, `REVIEW.md`, README, and nearby tests. Use the repo's domain words, not invented architecture labels.

Architecture vocabulary:
- Module: code with an interface and implementation.
- Interface: everything callers must know: types, invariants, errors, ordering, config, lifecycle.
- Depth: much behavior behind a small interface.
- Seam: where behavior can vary without editing callers.
- Adapter: concrete implementation at a seam.
- Locality: one behavior change touches few places.
- Leverage: callers get more capability than the interface cost.

Look for real friction: shallow pass-through modules, leaked invariants, many callers repeating one concept, test-only pure-function extraction with no locality, hard-to-test behavior, shotgun changes, fake seams with one adapter, or ADRs that no longer match reality.

Use the deletion test. If deleting a module makes complexity disappear, it was probably shallow. If deleting it spreads complexity across callers, it was earning depth.

Return only candidates backed by files and a concrete future change path. Do not propose generic cleanups, style, renames, layering theater, or new abstractions without leverage.

For each candidate include:
- Files: `<paths>`
- Friction: `<what is costly now>`
- Move: `<specific design move>`
- Payoff: `<locality/leverage/testability improvement>`
- Trade-off: `<accepted cost>`

Stop after candidates and ask which one to explore. If a decision should update `CONTEXT.md` or an ADR, ask explicitly before writing docs.
