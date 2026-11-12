---
name: understand-system
description: Understand one or more local codebases as a system. Resolves explicit targets and the active workspace first, then maps architecture, relationships, real flows, contracts, local runtime/debug context, validation seams, and evidence gaps.
---

# Understand System

Understand the system named in `$ARGUMENTS` or implied by the conversation. A system may be one repo, many repos, generated artifacts, scripts, local state, logs, configs, processes, and debugging context discovered from evidence.

Use only the current agent. Do not invoke subagents. This is read-only reconnaissance unless the user separately asks for changes.

Use this for how-does-this-work, how-do-these-repos-relate, where-is-the-contract, and how-do-I-debug-locally questions. To harvest reusable patterns, use `study-code` instead.

## Resolve the target

If the target is unclear after reading the conversation, ask one concise narrowing question and stop.

Resolution order:

1. Expand explicit paths and inspect existing paths directly.
2. Inspect the active workspace or current repository when it matches the target or no narrower path was named.
3. Follow locally evidenced related paths from workspace manifests, repository instructions, referenced files, and configuration.
4. Search existing bounded collection roots configured or conventional for the current environment. Treat source collections under a user workspace or home directory as optional fallbacks, not a required layout.
5. Match exact directory basenames before case-insensitive or prefix/suffix variants. Use fuzzy matches only to produce a short candidate list; do not guess silently.

Resolve evidence-derived paths and symlinks canonically, and follow relevant paths outside the repo. Do not recursively search an entire home directory when the steps above fail; ask for the path instead. Skip generated, vendored, dependency, cache, and build-output directories unless evidence says they are part of the system contract.

If multiple plausible matches remain, list them and ask one narrowing question. If no local match exists, say where you searched and what path or clone is needed.

## Safety

Treat local/internal code, logs, configs, and runtime state as confidential. Summarize rather than paste large excerpts; prefer paths, symbols, config keys, schema shapes, and short safe excerpts. Never expose secrets, tokens, private keys, customer or personal data, private payloads, or incident details; redact identifiers and credentials.

Stay read-only:

- Do not edit repos, generated files, configs, logs, caches, databases, lockfiles, or local state.
- Do not run destructive git, cleanup, reset, install, migration, write, or publish commands.
- Use bounded non-mutating commands only: `find`, `rg`, `grep`, `git status`, `git log`, package script listing, `--help`, config/schema inspection, and metadata reads.
- Do not start services, run broad tests, replay jobs, mutate databases, or call external production systems unless explicitly requested or clearly documented as safe and necessary.
- For files outside the repo, inspect metadata (existence, names, sizes, timestamps) before contents, read only small relevant excerpts, and state how the path was discovered.
- Ask before reading sensitive-looking or unrelated files such as credential stores, private keys, token files, production dumps, personal mail, or large customer/user payload logs.

## Investigation workflow

For each resolved repo/codebase:

1. Read local instructions first (`AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING`, README, architecture docs, ADRs, runbooks), including nested instructions for inspected paths.
2. Identify the repo role: service, CLI, app, library, worker, generated client, schema/protocol repo, infra/deployment repo, test harness, or support tool.
3. Read high-signal surfaces before details: workspace config, scripts, entrypoints, public APIs, schemas/contracts, config loading, state/storage, logging setup, representative callers, and tests. Search with the vocabulary they reveal.
4. Trace at least one real flow when possible:
   - Single repo: entrypoint → config/state/model → core module/service → boundary/client/storage → caller/UI/CLI/API → test, log, doc, or operational proof.
   - Multiple repos: repo A entrypoint → contract/client/schema/command → transport/config/local state → repo B boundary/handler/consumer → test, log, doc, or operational proof.
5. Stop when you can explain the operating model, important flows, debugging seams, and main unknowns.

## Wider local context

Do not hard-code product-specific paths. Derive runtime/debug context from docs, scripts, config defaults, logger setup, state/storage code, tests, and dev tooling. Look for logs and telemetry; config, env files, and env vars; state stores, caches, and databases; XDG/app-support/temp paths; sockets, pid files, locks, ports, and dev servers; generated artifacts and codegen outputs; and dev orchestration such as docker compose, Procfile, Tilt, mise, direnv, and bootstrap scripts.

## Multi-repo mode

When more than one codebase is in scope, search both directions for imports, workspace links, generated clients, shared types/schemas, HTTP/RPC/OpenAPI/protobuf/GraphQL contracts, queues/events/webhooks/jobs, CLI and subprocess calls, shared local files/sockets/ports/databases, env vars, feature flags, service names, and tests/docs/runbooks mentioning both sides.

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

For important relationships, trace one cross-repo flow. If none can be proven, state the strongest evidence and exact missing proof.

## Evidence standard

Separate conclusions into:

- **Evidence-backed**: supported by specific files, symbols, commands, tests, docs, logs, or runtime artifacts.
- **Likely but unproven**: plausible from partial evidence, but missing a caller, handler, test, doc, log, or runtime proof.
- **Unknown / not found**: searched for and not found, inaccessible, ambiguous, or outside the local checkout.

Do not claim interaction from name matches or infer architecture from directory layout alone. Do not trust README claims as current behavior unless code, config, tests, or runtime artifacts support them. Report conflicting evidence.

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

For small systems, shorten the report but keep evidence and unknowns. For large systems, rank the most important flows and relationships instead of listing everything. Include runtime or debug context, not only static architecture.
