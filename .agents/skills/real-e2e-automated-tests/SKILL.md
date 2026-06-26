---
name: real-e2e-automated-tests
description: "Create or harden automated real end-to-end tests. Use when the user wants durable E2E regression coverage, smoke or acceptance tests, CI/test-lane proof, or an automated test that boots the real system and hits public boundaries. The skill writes the test, wires it into the right lane, runs it, and proves it catches real regressions unless blocked by unavailable infrastructure."
---

## Validation reuse and check scope

Before running a slow, broad, external, stateful, or CI-equivalent command, check whether this conversation or current-SHA CI/artifacts already contain usable proof. Reuse prior passing evidence instead of rerunning only when it is visible, ran after the last relevant edit, covers the same command/scenario and behavior, edge case, or public boundary, and no touched file, config, dependency, fixture, generated output, runtime state, or environment assumption it depends on changed afterward. If uncertain, run the narrowest freshness check that resolves the uncertainty before escalating.

Default to the narrowest honest proof. Run broader suites, full builds, CI reruns, or live/E2E flows only when required by blast radius, merge/release policy, changed shared API/schema/build/test infrastructure/dependencies/auth/security/persistence/concurrency, merge/conflict integration risk, missing targeted seams, or because the broad command is the only proof that covers the behavior.

Final reports must distinguish reused proof, newly run commands, and checks intentionally not run.

Create an automated **real** end-to-end test for `scope`, `$ARGUMENTS`, recent changes, or the conversation target, and keep going until the test exists, is wired into the appropriate lane, and has been validated or is blocked by a concrete infrastructure limitation. Real means the production engine is running and the test drives the same public boundary a user, service, CLI, worker, or protocol client depends on.

This skill is for durable automated coverage. It is not the right skill for one-off live QA that operates a real system without writing tests or running CI/test lanes; use `real-e2e-live-check` for that.

This is not a planning skill. The plan/contract is only a short alignment checkpoint before implementation; it is not an acceptable final output. After writing the contract, immediately do the work: edit files, add or harden the E2E, wire scripts/CI when needed, run the relevant commands, fix failures caused by the new test, and prove the test would catch a real regression when safe.

Realness contract is binary: either the automated test drives the real public boundary, boots or reaches the required real engine/backends, is wired into the right lane, runs green, and catches a realistic regression when safe, or the work is blocked. Do not downgrade quietly to mocks, fake endpoints, internal helpers, unverified startup, broad sleeps, package-script-name guesses, or "likely works" claims. Confidence is achieved only for the exact checked contract; never claim 100% confidence for unobserved behavior, omitted edge cases, unwired tests, or unavailable infrastructure.

Persistence standard: continue discovery, editing, wiring, running, debugging, fixes, reruns, regression proof, artifact capture, and cleanup until the automated E2E contract is complete or a concrete blocker remains. Stop only for a named blocker such as missing credentials, missing tools, unavailable infrastructure, unclear auth/header behavior, unclear lane ownership, unsafe mutation, required approval, or a route that would use fakes inside the changed path. If the new E2E exposes a product issue from the current diff and fixing it is in scope, fix it and rerun the affected proof instead of ending at the failure.

Blocked is a last resort. A failed command, missing doc, ambiguous script, expired token, unavailable first-choice environment, red test, or unclear first lane is not yet a blocker. Before reporting blocked, perform blocker burn-down:
- expand discovery through nearby docs, repo-local skills, scripts, CI config, adjacent tests, smoke lanes, logs, Makefiles, package files, container/process config, and recent repo conventions;
- switch to the next faithful E2E route when the first route cannot work, instead of treating the first route as mandatory;
- inspect test failures, process logs, readiness probes, generated artifacts, port/process state, and config/env loading, then fix in-scope product, harness, local/tool/config, or wiring issues and retry;
- search repo-backed auth, credential, tenant/cloud, service-proxy, staging setup, and CI-lane ownership paths before assuming credentials or wiring are unavailable;
- request required approval for external access, long-running stacks, privileged commands, or safe deployed-resource mutation instead of self-blocking;
- ask one focused question only when it can unlock the last missing fact, credential, route choice, lane ownership, or approval. Report blocked only when no safe source-backed next action remains.

Gold standard: when relevant examples are available, inspect repo-local E2E docs, smoke scripts, package scripts, Makefiles, Docker/process startup, readiness helpers, and adjacent tests before designing the automated test. Prefer patterns that boot Docker/process stacks, wait for readiness, call HTTP/CLI/ACP/storage/browser boundaries, persist or observe downstream effects, collect artifacts, and prevent fake smoke lanes from creeping in.

## Repo Discovery Protocol

Do not choose an E2E approach until you can cite the repo-local source that owns startup, auth/header behavior, public boundary, readiness, cleanup, and test-lane wiring. Build a source-backed route map before editing:
- repo instructions and repo-local skills/runbooks read;
- changed behavior and public boundary;
- candidate E2E routes and why each is or is not faithful enough;
- startup/dependency source;
- auth, headers, tenant/cloud context, token, service-proxy, or deployed environment source;
- readiness/deepcheck/status-polling source;
- cleanup/artifact/log source;
- existing local command, package script, Makefile target, CI step, or smoke lane that owns this boundary;
- chosen route, explicit unknowns, and blocker threshold.

Prefer repo-local skills and runbooks over inferred commands. Search for skills/docs/scripts with names or descriptions that mention e2e, local, staging, dev-shard, smoke, runtime, browser, agent protocol, CLI, deploy, provider, auth, service-proxy, LocalStack, Docker, Compose, or Kind. Use automated tests and CI files as map sources for payloads, startup, auth, readiness, lane ownership, and edge cases, but do not assume a script is the right lane from its name alone.

Prefer repo-owned CLIs, SDK clients, browser drivers, or protocol clients over raw `curl` when they wrap auth, routing, request headers, tenant context, staging behavior, or service-proxy behavior. Never hand-roll deployed auth, Slauth, ASAP, tenant, service-proxy, staging, or production headers unless repo context proves that is the intended path.

Before adding a new E2E lane, prove no existing lane owns the boundary. A test that is not wired into a discoverable local and CI/test lane is incomplete. If no source-backed route exists after blocker burn-down, ask one focused question when user input can unlock progress; report the missing route as a blocker only when no safe source-backed next action remains.

Contract checkpoint before editing: write the realness contract in chat in no more than a few bullets, then implement. Name the public behavior, top-level API/protocol/CLI/browser/HTTP route, runtime/process/container/backends that must be live, observable success criteria, highest-risk failure mode, CI or test lane that will run it, allowed out-of-process simulators, and every forbidden shortcut. If any of these are unknown, inspect more or ask one focused question, then proceed as far as possible.

Edge-case requirement: before editing, identify a small edge-case matrix for the behavior under test:
- **Automated E2E**: cases covered by the automated test, including at least one highest-risk edge/negative case unless blocked by concrete infrastructure or safety limits.
- **Omitted**: meaningful cases intentionally omitted, each with a concrete reason.

Consider applicable invalid or malformed input, empty/minimal input, duplicate/repeated/idempotent calls, missing resources or not-found behavior, permission/auth/config boundaries, persistence across restart or cleanup/retry behavior, concurrency/ordering/timeout/partial-failure risks, and downstream/backend unavailability when safely simulatable outside the engine boundary. Prefer the highest-risk edge over exhaustive coverage. A happy-path-only E2E is incomplete unless the behavior truly has no meaningful edge case and the final answer explains why.

Non-negotiables:
- Hit a public product boundary. Do not import internals, call private helpers, or assert implementation details as the primary proof.
- Boot the real engine path: service, CLI, worker, runtime, container, persistence, queue, cache, object store, browser app, or protocol bridge as applicable.
- Use realistic data and verify durable outcomes: response body, file/database/object state, emitted event, downstream side effect, logs, screenshots, or protocol transcript.
- Use simulators only outside the engine boundary for uncontrollable third-party systems, and name them in the contract. They must speak the real protocol and must not replace the behavior under test.
- Wire the test into an existing E2E/smoke command and CI/test lane, or add the smallest adjacent lane needed for the same purpose.
- Add availability gating only for missing external infrastructure. Required CI mode must fail when the stack is unavailable.
- Capture cleanup and artifacts for failures without hiding failures.

Forbidden as E2E proof:
- mocks, fake adapters inside the engine path, in-memory replacements for real backends under test, mocked network clients, or edge checks performed only against mocks/fakes instead of the real public boundary;
- test-only endpoints, bypass flags that skip the real path, broad sleeps instead of readiness probes, skipped tests, weakened assertions, or snapshot-only checks;
- "server starts" checks, package-script-name guesses, hand-rolled deployed headers, happy-path-only coverage when meaningful edge cases exist, or tests whose main assertion is that a fake was called.

Do not stop at ambiguity until blocker burn-down is complete. Then ask one focused question when multiple plausible routes exist and repo docs do not rank them; auth/header/token behavior is unclear; startup, readiness, or cleanup ownership is unknown; the public boundary is unknown; the only discovered route uses mocks/fakes inside the changed path; or a new CI/test lane would be required but lane discovery/wiring is unclear. Report blocked only when no question, approval, route switch, log investigation, or in-scope fix can progress the real E2E.

Implementation loop:
1. Build the source-backed route map. Study existing automated E2E lanes, package scripts, CI config, Docker/process startup, readiness helpers, adjacent tests, and repo-documented real run/E2E instructions such as README files, docs, Makefiles, package scripts, compose files, or smoke scripts. Reuse the closest lane instead of inventing a parallel harness.
2. Add the smallest high-signal automated test that drives the real boundary from outside the engine and observes the business outcome. The automated E2E must include at least one meaningful edge/negative assertion through the same real public boundary unless blocked by concrete infrastructure or safety limits; choose the highest-risk edge from the matrix rather than broad exhaustive coverage.
3. Wire the test into the existing E2E/smoke command and CI/test lane, or add the smallest adjacent lane needed for the same purpose.
4. Add or update a policy/guardrail test when needed to prevent future drift back to fakes, internal imports, or untracked CI wiring.
5. Run or reuse fresh proof for the exact automated E2E command and the nearest broader check required by the validation policy. Reuse is valid only when the same lane/scenario, current effective diff, real boundary, runtime/backends, edge or negative case, and CI/test-lane wiring were proven after the last relevant edit. If stale, rerun the narrowest E2E lane first. Fix failures caused by the new test or wiring. If infrastructure is unavailable locally, perform blocker burn-down, run the non-network policy/compile checks that still apply, and state the missing command as blocked only when no safe source-backed next action remains.
6. Prove or reuse fail/pass proof that the automated E2E would catch a realistic regression when safe: make one temporary reversible break in production code, config, or fixture; confirm the E2E fails for the expected reason; restore; rerun green. Do not repeat this if the same realistic regression was already observed failing and restored passing for the current effective diff. If unsafe, explain the exact reason.

Completion gate before final: reread the automated E2E contract and answer each gate yes/no. If any gate is no, keep going or report blocked only after blocker burn-down; never produce a complete confidence line.
- Source-backed route map completed, with no unresolved realness-critical unknown.
- Automated test drives the real public boundary, and no fake, mock, internal helper, bypass flag, or test-only endpoint replaced the changed path.
- Real engine, process, container, protocol bridge, backend, persistence, queue, cache, object store, or browser app was booted or reached as the contract requires.
- Readiness was proven by a source-backed probe, not broad sleeps or process-start claims.
- Test is wired into the discoverable local E2E/smoke command and CI/test lane, or missing lane ownership is named as the blocker.
- Main assertion observes durable product outcome through the real boundary.
- Highest-risk edge or negative assertion passed through the same boundary, or omission has a concrete reason.
- Realistic regression proof failed for the expected reason and then reran green after restore, or equivalent fresh prior fail/pass proof was reused, unless unsafe and explicitly justified.
- Any failure caused by the current diff was fixed and rerun, or is named as the blocker.

Final only after implementation: contract, edge-case matrix, boundary hit, stack/backends booted, CI/test lane, automated E2E result, automated edge/negative case covered, regression caught, commands/results, changed files, fixes made from discovered caused issues, edge cases intentionally omitted with reasons, and any remaining blocker.

Never end with only a plan. Never call it real E2E unless it would fail when the real engine behavior is broken. The final confidence line must be either `Confidence: complete for the stated automated E2E contract` or `Confidence: not achieved because <blocker>`.
