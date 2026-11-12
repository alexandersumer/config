---
name: study-code
description: Study locally available OSS or Atlassian repositories with the current agent to extract excellent, evidence-backed implementation and architecture patterns for a requested or inferred aspect.
---

# Study Code

Study the aspect in `$ARGUMENTS`, else infer the aspect from the conversation, active design, current implementation need, or product context. Use only the current agent; do not invoke subagents. Read-only: do not edit inspected repositories unless the user separately asks for changes.

## Resolve the study target

If the aspect is unclear after reading the conversation, ask one narrowing question and stop. Example aspects: error handling, typed boundaries, Effect service layering, CLI architecture, API clients, testing seams, fixture design, dependency injection, configuration loading, plugin systems, state machines, storage schemas, migrations, observability, rollout paths, package layout, and domain modeling.

Decide the source: OSS, internal Atlassian, or both, from the user's words or the candidate repositories. Apply the internal rules below to any internal repository.

Resolve candidate repositories in this order:

1. Explicit repository paths or names from the user or conversation.
2. The active workspace or current repository when it is relevant to the aspect.
3. Locally evidenced or configured bounded collection roots from workspace manifests, repository instructions, harness workspace metadata, or existing configuration.
4. Other bounded source collections explicitly exposed by the current environment.

Do not assume a named home-directory layout or recursively crawl the home directory. If no bounded candidate location can be resolved, ask for a repository or collection path.

Choose a small candidate set before deep reading: prefer mature, production-proven, well-tested, well-documented repositories with typed boundaries, clear ownership, and architecture relevant to the aspect. Avoid toy, abandoned, generated, vendored, stale-experiment, or low-signal repositories and broad monorepo wandering. If many match, pick the 1-3 strongest and say why.

## Internal code

Treat internal code as confidential. Summarize instead of pasting large excerpts, citing local paths, symbols, and tests the user can inspect. Never expose secrets, tokens, customer data, private operational payloads, internal URLs, or incident details unless directly relevant and safe. Prefer transferable principles over internal-specific names. If a pattern depends on Atlassian-only infrastructure, state that dependency instead of presenting it as generally portable.

## Exploration workflow

1. Read local instructions first (`AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING`, README, architecture docs, ADRs), including nested ones, to interpret conventions, not to expand scope.
2. Read high-signal surfaces before implementation details: package/workspace config, entry points, public APIs, SPI boundaries, schemas, core interfaces, representative callers, and tests around the aspect.
3. Search narrowly with aspect terms, for example errors (`Result`, `TaggedError`, `catchTag`, `Diagnostic`), Effect (`Context.Tag`, `Layer`, `Effect.fn`, `Schema.decodeUnknown`), tests (`it.effect`, `fixture`, `golden`, `snapshot`, `harness`), config (`schema`, `parse`, `validate`, `redacted`), boundaries (`interface`, `port`, `adapter`, `service`, `client`, `spi`).
4. Follow one or two real flows end to end: entry point → boundary/schema → abstraction → implementation → caller → test or operational check. Prefer patterns proven by production usage and tests.
5. Filter hard. Drop findings that are merely stylistic, fashionable, unsupported by tests or real callers, too framework- or infrastructure-specific to transfer, clever without a clear payoff, or too large to explain accurately.
6. Stop when you have enough evidence for up to seven high-quality patterns. One or two is correct when only one or two survive the filter; do not keep searching or lower the bar to fill a quota.

## Quality bar

Call a pattern excellent only when file-backed evidence shows it reduces total complexity, hides a real decision, protects an invariant, improves locality, gives callers leverage, creates a useful test seam, improves operability, or makes future change cheaper. Extract small transferable principles and their constraints; do not recommend cargo-culting repo-specific structure. Include anti-patterns only when they clarify why the selected pattern is better.

## Output

Return a compact ranked report, not a code dump. For each pattern include:

- `Pattern`: short descriptive name.
- `Where found`: repo, path, symbol/test when available.
- `What it does`: the concrete mechanism, summarized safely for internal code.
- `Why it is excellent`: the complexity, invariant, locality, seam, or operability win.
- `Transferable principle`: the general lesson independent of the repo and its infrastructure.
- `Minimal version to copy`: the smallest useful adaptation, without leaking sensitive details.
- `When not to use it`: constraints and trade-offs.
- `Caveats / internal assumptions`: for internal code, Atlassian-specific dependencies or assumptions.
- `Evidence`: concise local references; tiny snippets only when safe and materially helpful.

If no high-quality pattern survives the filter, say so and name the repositories or paths inspected.
