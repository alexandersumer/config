---
name: real-e2e-live-check
description: "Operate a real running system end to end without writing automated tests or running CI/test lanes. Use when the user wants live E2E QA of the current effective diff through local real services, agent protocol clients, CLI, browser, HTTP APIs, staging/dev shards, or deployed resources. The skill discovers the most appropriate real-enough environment, drives the public boundary, covers relevant edge cases, captures evidence, cleans up, and reports blockers honestly."
---

## Validation reuse and check scope

Before running a slow, broad, external, stateful, or CI-equivalent command, check whether this conversation or current-SHA CI/artifacts already contain usable proof. Reuse prior passing evidence instead of rerunning only when it is visible, ran after the last relevant edit, covers the same command/scenario and behavior, edge case, or public boundary, and no touched file, config, dependency, fixture, generated output, runtime state, or environment assumption it depends on changed afterward. If uncertain, run the narrowest freshness check that resolves the uncertainty before escalating.

Default to the narrowest honest proof. Run broader suites, full builds, CI reruns, or live/E2E flows only when required by blast radius, merge/release policy, changed shared API/schema/build/test infrastructure/dependencies/auth/security/persistence/concurrency, merge/conflict integration risk, missing targeted seams, or because the broad command is the only proof that covers the behavior.

Final reports must distinguish reused proof, newly run commands, and checks intentionally not run.

Operate the current effective diff through a **real running system** and prove the behavior works end to end without creating automated tests and without running CI/test lanes. Real means a user, operator, service, CLI, browser, protocol client, or API caller drives the same public boundary the product depends on while the relevant runtime, service, worker, container, persistence, queue, cache, object store, browser app, or deployed resource is live.

This skill is for live/runtime QA evidence. It is not the right skill when the user wants durable automated E2E regression coverage, CI proof, or a new smoke/acceptance test; use `real-e2e-automated-tests` for that.

Realness contract is binary: either the real public boundary was operated and every contract item has evidence, or the work is blocked. Do not downgrade quietly to mocks, fake endpoints, partial startup, internal helpers, unauthenticated shortcuts, unverified readiness, broad sleeps, or "likely works" claims. Confidence is achieved only for the exact checked contract; never claim 100% confidence for unobserved behavior, omitted edge cases, or unavailable infrastructure.

Persistence standard: continue discovery, setup, operation, debugging, fixes, reruns, evidence capture, and cleanup until the live-check contract is complete or a concrete blocker remains. Stop only for a named blocker such as missing credentials, missing tools, unavailable infrastructure, unclear auth/header behavior, unsafe mutation, required approval, or a route that would use fakes inside the changed path. If a product issue from the current diff is found and fixing it is in scope, fix it and rerun the affected proof instead of ending at the failure.

Blocked is a last resort. A failed command, missing doc, ambiguous script, expired token, unavailable first-choice environment, or initial auth/setup error is not yet a blocker. Before reporting blocked, perform blocker burn-down:
- expand discovery through nearby docs, repo-local skills, scripts, CI/test files as map sources, adjacent tests, logs, Makefiles, package files, container/process config, and recent repo conventions;
- switch to the next faithful route on the environment ladder when the first route cannot work, instead of treating the first route as mandatory;
- inspect logs, command output, readiness endpoints, generated artifacts, port/process state, and config/env loading, then fix in-scope local/tool/config issues and retry;
- search repo-backed auth, credential, tenant/cloud, service-proxy, and staging setup paths before assuming credentials are unavailable;
- request required approval for external access, long-running stacks, privileged commands, or safe deployed-resource mutation instead of self-blocking;
- ask one focused question only when it can unlock the last missing fact, credential, route choice, or approval. Report blocked only when no safe source-backed next action remains.

Do not be repo-prescriptive. Discover the right approach from the current repository and task: `AGENTS.md`, README/CONTRIBUTING, local `.agents/skills` or `.rovodev/skills`, package scripts, Makefiles, compose files, runbooks, smoke scripts, E2E docs, deployment docs, CLI docs, API docs, and existing local/deployed workflow notes. You may read automated tests and CI scripts to understand startup, boundaries, fixtures, and edge cases, but do not write tests or use a test lane as the proof for this skill.

## Repo Discovery Protocol

Do not choose a live-check approach until you can cite the repo-local source that owns startup, auth/header behavior, public boundary, readiness, and cleanup. Build a source-backed route map before operating anything:
- repo instructions and repo-local skills/runbooks read;
- changed behavior and public boundary;
- candidate live routes and why each is or is not faithful enough;
- startup/dependency source;
- auth, headers, tenant/cloud context, token, service-proxy, or deployed environment source;
- readiness/deepcheck/status-polling source;
- cleanup/artifact/log source;
- chosen route, explicit unknowns, and blocker threshold.

Prefer repo-local skills and runbooks over inferred commands. Search for skills/docs/scripts with names or descriptions that mention e2e, local, staging, dev-shard, smoke, runtime, browser, agent protocol, CLI, deploy, provider, auth, service-proxy, LocalStack, Docker, Compose, or Kind. Use automated tests and CI files as map sources for payloads, startup, auth, readiness, cleanup, and edge cases, but do not run them as the live-check proof.

Prefer repo-owned CLIs, SDK clients, browser drivers, or protocol clients over raw `curl` when they wrap auth, routing, request headers, tenant context, staging behavior, or service-proxy behavior. Never hand-roll deployed auth, Slauth, ASAP, tenant, service-proxy, staging, or production headers unless repo context proves that is the intended path.

Do not proceed from package-script names alone. Do not treat process start as readiness. If no source-backed route exists after blocker burn-down, ask one focused question when user input can unlock progress; report the missing route as a blocker only when no safe source-backed next action remains.

Before operating anything, write a short live-check contract in chat:
- changed behavior and current effective diff being checked;
- public boundary to drive: browser, CLI, agent protocol client, SDK, HTTP API, worker trigger, webhook, deployed endpoint, or another real user/operator surface;
- chosen environment and why it is real enough: existing local process, repo-documented local real stack, dev shard, staging, or deployed resource;
- safety class: local only, staging/dev mutating, production read-only, or production mutating;
- required tools, auth, ports, credentials, resources, and approval needs;
- main success path, highest-risk edge probes, evidence to capture, cleanup plan, and explicit forbidden shortcuts.

Environment selection ladder:
1. Use an already-running local system when it is clearly the correct composition root and reaches the changed public boundary.
2. Prefer the repo-documented local real stack when it faithfully exercises the changed path: local servers, Docker Compose, Kind, Lima, local storage/cache/object stores, local provider stacks, real CLIs, browser automation, or protocol bridges.
3. Use the real client surface that matches the changed behavior. Choose an agent protocol client only when the behavior is observable through the agent protocol; choose browser for UI flows, CLI for operator flows, SDK/API clients for programmatic flows, and direct HTTP only when that is the product boundary.
4. Use a dev shard or staging resource when local cannot faithfully cover the behavior, such as deployed auth, TLS/ALB, service mesh, multi-node scheduling, production image wiring, network policy, architecture-specific runtime behavior, published catalog/config, or a bug that only appears on deployed paths.
5. Use production only with explicit user approval. Prefer read-only probes; for any production mutation, require a narrow resource scope, unique names, rollback/cleanup, and clear user authorization before acting.

Live-check proof must include:
- preflight that the target environment, auth, tools, ports, and public boundary are actually reachable;
- readiness checks for the real boundary, not broad sleeps or "process started" claims;
- one main user/operator/client scenario through the public boundary;
- at least one meaningful edge or negative probe unless there is no relevant edge and the final explains why;
- durable evidence such as response bodies, resource IDs, object/file/database state, UI screenshots or snapshots, logs, events, protocol transcript, command output, poll history, or cleanup confirmation;
- cleanup of resources and processes you started, or an explicit reason they were intentionally left running.

Choose edge probes from the risk in the current diff: invalid or minimal input, not found, auth/permission/config failure, repeated/idempotent call, readiness race, timeout/retry, cleanup failure, persistence/durable state, routing, downstream dependency behavior, or post-operation health. Prefer the highest-risk edge over exhaustive coverage.

Forbidden proof:
- writing automated tests, adding test fixtures, wiring CI/test lanes, or claiming a test run is the live-check result;
- mocks, fake adapters inside the changed path, test-only endpoints, bypass flags, broad sleeps, skipped/ignored failures, or assertions only against internals;
- package-script-name guesses, hand-rolled deployed headers, or using an agent protocol client, browser, CLI, or HTTP just because it is convenient when another public boundary is the real user surface;
- mutating shared deployed resources without a safety class, unique identifiers, cleanup plan, and required approval;
- downgrading to fake/local-only evidence when the contract requires deployed infrastructure. Report blocked instead.

Do not stop at ambiguity until blocker burn-down is complete. Then ask one focused question when multiple plausible routes exist and repo docs do not rank them; auth/header/token behavior is unclear; startup, readiness, or cleanup ownership is unknown; the public boundary is unknown; the only discovered route uses mocks/fakes inside the changed path; the only discovered command is a CI/test lane; or production/shared-resource mutation would be required without clear approval and cleanup. Report blocked only when no question, approval, route switch, log investigation, or in-scope fix can progress the real check.

Implementation loop:
1. Build the source-backed route map. Inspect the effective diff, relevant docs/skills/scripts, and available client surfaces until the real-enough approach is clear. If the safe environment or public boundary remains unclear, ask one focused question.
2. Write the live-check contract and safety classification in chat before starting servers, creating resources, or hitting deployed systems.
3. Start or connect to the chosen real environment using repo-documented commands where available. Keep process IDs, ports, URLs, resource IDs, and log locations for cleanup and reporting. Request approval before external mutation, production access, expensive resources, or long-running stacks when required.
4. Run or reuse preflight/readiness evidence under the validation policy. For live systems, reuse prior full-flow evidence only when the same public boundary, runtime/deployed resource, inputs, state, and expected observations were checked after the last relevant edit; when current liveness matters, run only the cheapest readiness/status/freshness probe that makes the reused evidence honest. If the environment is unavailable, missing auth, or unsafe, perform blocker burn-down before stopping with a concrete blocker; never substitute a fake path.
5. Drive or reuse fresh full-flow evidence for the main scenario through the public boundary and capture the durable result. Reuse is valid only when the same public boundary, environment/resource, inputs, durable observations, and current effective diff are covered.
6. Drive or reuse fresh full-flow evidence for the selected edge probes through the same public boundary when safe. Check or reuse fresh proof for post-operation health or invariants when the flow can leave state behind.
7. Clean up resources/processes you created. For reused evidence, confirm prior cleanup evidence or name remaining state and why.
8. If the live check exposes a product issue in the current diff and fixing it is in scope, fix it, then rerun or reuse current-SHA proof for the affected scenario and edge probe. Do not hide caused failures.

Completion gate before final: reread the live-check contract and answer each gate yes/no. If any gate is no, keep going or report blocked only after blocker burn-down; never produce a complete confidence line.
- Source-backed route map completed, with no unresolved realness-critical unknown.
- Real public boundary operated, or fresh same-scope full-flow evidence was reused, and no fake, mock, internal helper, bypass flag, or test lane replaced the changed path.
- Real environment was reached or booted, or fresh same-scope environment evidence was reused, and readiness was proven by a source-backed probe.
- Main scenario passed through the real boundary with durable evidence, newly captured or validly reused.
- Highest-risk edge or negative probe passed through the same boundary, or fresh same-scope evidence was reused, or omission has a concrete reason.
- Post-operation health, durable state, or invariants were checked or validly reused when the flow can leave state behind.
- Created resources and processes were cleaned up, prior cleanup evidence was reused, or remaining state is named with reason.
- Any failure caused by the current diff was fixed and rerun, fixed with current-SHA proof reused, or is named as the blocker.

Final only after live operation or valid reused full-flow evidence: contract, boundary used, environment chosen, safety class, preflight/readiness result, main scenario evidence, edge probes and outcomes, artifacts/logs/screenshots/transcripts, cleanup result, fixes made from caused issues, edge cases intentionally omitted with reasons, and remaining blockers.

Never call a live check complete if the real boundary was neither operated nor covered by valid same-scope full-flow evidence. Say blocked when credentials, tools, approvals, or infrastructure prevent the required proof. The final confidence line must be either `Confidence: complete for the stated live-check contract` or `Confidence: not achieved because <blocker>`.
