---
name: study-code-atlassian
description: Study locally available Atlassian repositories with the current agent to extract excellent, evidence-backed internal implementation and architecture patterns for a requested or inferred aspect.
---

# Study Code: Atlassian

Study `aspect`, else `$ARGUMENTS`, else infer the aspect from the conversation, active design, current implementation need, or Atlassian product context. Use only the current agent. Do not invoke subagents. Do not edit inspected repositories; this is a read-only research skill unless the user separately asks for changes.

## Resolve the study target

If the aspect is unclear after reading the conversation, ask one narrowing question and stop. Good aspects include Effect service layering, typed boundaries, error handling, API clients, storage schemas, migrations, testing seams, fixture design, dependency injection, configuration loading, product integration boundaries, observability, rollout paths, package layout, and domain modeling.

Resolve candidate repositories in this order:

1. Explicit repository paths or names from the user or conversation.
2. The active workspace or current repository when it is an Atlassian codebase relevant to the aspect.
3. Locally evidenced or configured bounded collection roots from workspace manifests, repository instructions, harness workspace metadata, or existing configuration.
4. Other bounded source collections explicitly exposed by the current environment.

Do not assume a named home-directory layout or recursively crawl the home directory. If no bounded candidate location can be resolved, ask for an Atlassian repository or collection path.

Choose a small candidate set before deep reading. Prefer production-mature repositories with strong local instructions, tests, typed boundaries, clear ownership, documented conventions, and architecture that is relevant to the aspect. Avoid broad monorepo wandering, generated code, vendored code, stale experiments, and low-signal repositories. If many repos match, pick the 1-3 strongest and say why before deep inspection.

## Confidentiality posture

Treat internal code as confidential context. Do not paste large internal code excerpts. Summarize implementation details and cite local paths, symbols, and tests so the user can inspect them in their checkout. Never expose secrets, tokens, customer data, private operational payloads, internal URLs, or incident details unless directly relevant and safe. Prefer transferable principles over internal-specific names. If a pattern depends on Atlassian-only infrastructure, state that dependency instead of presenting it as generally portable.

## Exploration workflow

1. Check local instructions first when present: `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING`, README, architecture docs, ADRs, and development docs. Follow nested instructions for any repo you inspect.
2. Read high-signal surfaces before implementation details: package/workspace config, service entry points, public APIs, SPI boundaries, schema files, core interfaces, representative callers, and tests around the requested aspect.
3. Search narrowly with terms from the aspect. For example: Effect (`Context.Tag`, `Layer`, `Effect.fn`, `Schema.decodeUnknown`), errors (`TaggedError`, `catchTag`, `Result`), tests (`it.effect`, `fixture`, `harness`, `integration`), config (`Config`, `redacted`, `schema`, `validate`), boundaries (`service`, `client`, `adapter`, `spi`, `port`).
4. Follow one or two real flows end to end: entry point → boundary/schema → service/interface → implementation → caller → test or operational check. Prefer patterns proven by production usage and tests over isolated clever code.
5. Filter hard. Drop findings that are merely stylistic, fashionable, unsupported by tests or real callers, too infrastructure-specific to transfer, clever without a clear payoff, or too large to explain accurately.
6. Stop when you have enough evidence for up to seven high-quality patterns. One or two is correct when only one or two survive the filter; do not keep searching or lower the bar to fill a quota.

## Quality bar

Call a pattern excellent only when file-backed evidence shows it reduces total complexity, hides a real decision, protects an invariant, improves locality, gives callers leverage, creates a useful test seam, improves operability, or makes future change cheaper. Prefer small transferable ideas over wholesale architecture copying. Extract the principle and constraints; do not recommend cargo-culting Atlassian-specific structure. Include caveats for platform assumptions. Include anti-patterns only when they clarify why the selected pattern is better.

## Output

Return a compact ranked report, not a code dump. For each pattern include:

- `Pattern`: short descriptive name.
- `Where found`: repo, path, symbol/test when available.
- `What it does`: the concrete mechanism, summarized safely.
- `Why it is excellent`: the complexity, invariant, locality, seam, or operability win.
- `Transferable principle`: the general lesson independent of internal infrastructure.
- `Minimal internal-safe version`: the smallest useful adaptation without leaking sensitive details.
- `When not to use it`: constraints and trade-offs.
- `Caveats / internal assumptions`: Atlassian-specific dependencies or assumptions.
- `Evidence`: concise local references; tiny snippets only when safe and materially helpful.

If no high-quality pattern survives the filter, say so and name the repositories or paths inspected.
