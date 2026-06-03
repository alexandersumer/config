---
name: study-code-oss
description: Study local OSS repositories in ~/oss with the current agent to extract excellent, evidence-backed implementation and architecture patterns for a requested or inferred aspect.
---

# Study Code: OSS

Study `aspect`, else `$ARGUMENTS`, else infer the aspect from the conversation, active design, or current implementation need. Use only the current agent. Do not invoke subagents. Do not edit repositories under `~/oss`; this is a read-only research skill unless the user separately asks for changes.

## Resolve the study target

If the aspect is unclear after reading the conversation, ask one narrowing question and stop. Good aspects include error handling, CLI architecture, testing seams, fixture design, dependency injection, configuration loading, API boundaries, plugin systems, state machines, migrations, observability, package layout, and type-safe domain modeling.

If the user names repos or paths, inspect those. Otherwise, search `~/oss` for candidates and choose a small set before deep reading. Prefer mature, active, well-tested, well-documented repos that are idiomatic for their language and relevant to the aspect. Avoid random, toy, abandoned, generated, vendored, or low-signal repositories. If many repos match, pick the 1-3 strongest and say why before deep inspection.

## Exploration workflow

1. Check local repo instructions first when present: `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING`, README, architecture docs, ADRs, and development docs. Use them to interpret conventions, not to expand scope.
2. Read high-signal surfaces before implementation details: package/workspace config, public API entry points, core interfaces, representative callers, and tests around the requested aspect.
3. Search narrowly with terms from the aspect. For example: errors (`Result`, `Error`, `Diagnostic`, `catch`, `raise`), tests (`fixture`, `golden`, `snapshot`, `harness`, `integration`), config (`schema`, `parse`, `validate`, `env`), boundaries (`interface`, `port`, `adapter`, `service`, `client`).
4. Follow one or two real flows end to end: public entry point → abstraction → implementation → caller → test. Prefer patterns proven by usage over isolated clever code.
5. Filter hard. Drop findings that are merely stylistic, fashionable, unsupported by tests or real callers, too framework-specific to transfer, clever without a clear payoff, or too large to explain accurately.
6. Stop when you have enough evidence for 3-7 high-quality patterns. Do not keep searching just because more repositories exist.

## Quality bar

Call a pattern excellent only when file-backed evidence shows it reduces total complexity, hides a real decision, protects an invariant, improves locality, gives callers leverage, creates a useful test seam, improves operability, or makes future change cheaper. Prefer small transferable ideas over wholesale architecture copying. Extract the principle and constraints; do not recommend cargo-culting repo-specific structure. Include anti-patterns only when they clarify why the selected pattern is better.

## Output

Return a compact ranked report, not a code dump. For each pattern include:

- `Pattern`: short descriptive name.
- `Where found`: repo, path, symbol/test when available.
- `What it does`: the concrete mechanism.
- `Why it is excellent`: the complexity, invariant, locality, seam, or operability win.
- `Transferable principle`: the general lesson independent of the repo.
- `Minimal version to copy`: the smallest useful adaptation.
- `When not to use it`: constraints and trade-offs.
- `Evidence`: concise references or a tiny snippet only when it materially helps.

If no high-quality pattern survives the filter, say so and name the repositories or paths inspected.
