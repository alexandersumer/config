---
name: understand-system
description: Understand one or more local codebases as a system. Searches ~/oss, ~/atlassian, and ~/src, then maps architecture, relationships, real flows, contracts, local runtime/debug context, validation seams, and evidence gaps.
register_cmd: true
---

# Understand System

Understand `system`, `repos`, `paths`, `$ARGUMENTS`, or the system implied by the conversation. A system may be one repo, many repos, generated artifacts, scripts, local state, logs, configs, processes, and debugging context discovered from evidence.

Use only the current agent. Do not invoke subagents. Do not edit inspected repos or local runtime files. This is read-only reconnaissance unless the user separately asks for changes.

Use this for questions like: how does this work, how do these repos relate, where is the contract, how do I debug this locally, what logs/config/state matter, or what should I read next. Do not use it just to harvest reusable patterns; use `study-code-atlassian` or `study-code-oss` for that.

## Resolve the target

If the target is unclear after reading the conversation, ask one concise narrowing question and stop.

Search for named codebases under these roots unless explicit paths are provided:

- `~/oss`
- `~/atlassian`
- `~/src`

Resolution order:

1. Expand explicit paths and inspect existing paths directly.
2. Match exact directory basenames under the search roots.
3. Match case-insensitive and common prefix/suffix variants.
4. Use fuzzy matches only to produce a short candidate list; do not guess silently.

Detect repo roots by `.git`, README, package/workspace files, language/build config, or clear source layout. Prefer shallow repo-like directories before deep filesystem walks. Avoid generated, vendored, dependency, cache, and build-output directories unless evidence says they are part of the system contract.

If multiple plausible matches remain, list them and ask one narrowing question. If no local match exists, say where you searched and what path or clone is needed.

## Safety

Treat local/internal code, logs, configs, and runtime state as confidential.

- Summarize; do not paste large private code/log/config excerpts.
- Never expose secrets, tokens, private keys, customer data, personal data, private payloads, or incident details.
- Prefer paths, symbols, config keys, schema shapes, timestamps, and short safe excerpts.
- Redact obvious identifiers and credentials.

Stay read-only:

- Do not edit repos, generated files, configs, logs, caches, databases, lockfiles, or local state.
- Do not run destructive git, cleanup, reset, install, migration, write, or publish commands.
- Use bounded non-mutating commands only when they improve understanding: `find`, `rg`, `grep`, `git status`, `git log`, package script listing, `--help`, config/schema inspection, and metadata reads.
- Do not start services, run broad tests, replay jobs, mutate databases, or call external production systems unless explicitly requested or clearly documented as safe and necessary.
- Ask before reading sensitive-looking files such as credential stores, private keys, token files, production dumps, personal mail, or large customer/user payload logs.

## Investigation workflow

For each resolved repo/codebase:

1. Read local instructions first: `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING`, README, architecture docs, ADRs, development docs, and runbooks. Follow nested instructions for inspected paths.
2. Identify the repo role: service, CLI, app, library, worker, generated client, schema/protocol repo, infra/deployment repo, test harness, or support tool.
3. Read high-signal surfaces before details: workspace config, package scripts, service/CLI entrypoints, public APIs, schemas/contracts, config loading, state/storage model, logging/observability setup, representative callers, and tests.
4. Search with discovered vocabulary: repo names, package names, commands, ports, env vars, endpoint paths, protocol names, schema names, generated clients, and domain terms.
5. Trace at least one real flow when possible:
   - Single repo: entrypoint → config/state/model → core module/service → boundary/client/storage → caller/UI/CLI/API → test, log, doc, or operational proof.
   - Multiple repos: repo A entrypoint → contract/client/schema/command → transport/config/local state → repo B boundary/handler/consumer → test, log, doc, or operational proof.
6. Prefer callers, tests, runtime wiring, and operational artifacts over isolated declarations.
7. Stop when you can explain the operating model, important flows, debugging seams, and main unknowns. Do not keep searching just because more files exist.

## Wider local context

Do not hard-code product-specific paths. Derive local runtime/debug context from evidence in docs, scripts, config defaults, logger setup, state/storage code, tests, fixtures, and dev tooling.

Look for hints such as:

- logs, traces, telemetry, logger transports
- config, settings, profiles, rc files, env files, env vars, defaults
- state, data, stores, caches, sqlite/databases, file stores
- home-directory helpers, XDG config/cache/data, app-support paths, temp dirs
- sockets, pid files, locks, ports, localhost URLs, dev servers
- generated artifacts, schemas, clients, codegen outputs
- docker compose, Procfile, Tilt, mise, direnv, bootstrap, dev scripts

When a local path outside the repo is discovered:

1. Inspect metadata first: existence, filenames, sizes, timestamps, directory shape.
2. Read only small relevant excerpts.
3. Redact secrets and private payloads.
4. State how the path was discovered.
5. Ask before reading if it appears sensitive or unrelated.

## Multi-repo mode

When more than one codebase is in scope, map relationships explicitly. Search both directions for:

- imports, workspace links, generated clients, shared libraries/types/schemas
- HTTP/RPC endpoints, OpenAPI, protobuf, GraphQL, JSON schema, typed contracts
- queues, topics, events, webhooks, jobs, workers
- CLI invocations, subprocess calls, shell scripts, dev orchestration
- shared local files, sockets, ports, databases, caches, config names
- env vars, feature flags, deployment/service names
- tests, fixtures, docs, logs, or runbooks mentioning both sides

Classify each relationship:

- direct runtime dependency
- protocol/API integration
- CLI-to-service or tool-to-tool integration
- shared package/library/type/schema
- generated-code relationship
- shared local state/config/logging
- dev-workflow or orchestration relationship
- deployment/ops relationship
- conceptual only / no direct evidence found

For important relationships, trace one cross-repo flow. If no cross-repo flow can be proven, state the strongest evidence and exact missing proof.

## Evidence standard

Separate conclusions into:

- **Evidence-backed**: supported by specific files, symbols, commands, tests, docs, logs, or runtime artifacts.
- **Likely but unproven**: plausible from partial evidence, but missing a caller, handler, test, doc, log, or runtime proof.
- **Unknown / not found**: searched for and not found, inaccessible, ambiguous, or outside the local checkout.

Do not claim interaction from name matches alone. Do not infer architecture from directory layout alone. Do not trust README claims as current behavior unless code, config, tests, or runtime artifacts support them. If evidence conflicts, report the conflict.

## Output

Return a compact operator map, not a code dump. Include only applicable sections.

```markdown
# Understanding: <system or repo names>

## Scope inspected
- Repos / paths:
- Search roots:
- Important instructions/docs:
- Wider local context discovered:

## Executive model
<what the system is, what each major part does, and how the pieces fit>

## Repo map
### <repo or path>
- Role:
- Main entrypoints:
- Core modules / boundaries:
- Runtime config/state/logging:
- Tests / validation seams:

## Relationship map
- <relationship>: <classification, mechanism, evidence>

## Flows traced
### <flow name>
- Entry:
- Path:
- Boundary / contract:
- Evidence:
- Validation or debug seam:

## Local runtime/debug context
- <path or artifact>: how discovered, what it is used for, safe inspection notes

## Evidence-backed
- <fact with path/symbol/test/log reference>

## Likely but unproven
- <hypothesis and missing proof>

## Unknowns / next reads
- <gap, what was searched, next best evidence source>
```

For small systems, shorten the report but keep evidence and unknowns. For large systems, rank the most important flows and relationships instead of listing everything.

## Anti-patterns

- README-only summaries.
- Grep-only relationship claims.
- Hard-coded local paths or product conventions.
- Large private code/log/config dumps.
- Wandering into unrelated repos from common-word matches.
- Broad builds, installs, service starts, or tests before understanding scripts and safety.
- Static architecture summaries with no runtime/debug context.
- Pattern harvesting instead of system understanding.
