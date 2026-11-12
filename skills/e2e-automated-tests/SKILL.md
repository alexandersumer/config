---
name: e2e-automated-tests
description: "Create or harden real automated end-to-end tests. Use when the user wants durable E2E regression coverage, smoke or acceptance tests, CI/test-lane proof, or an automated test that boots the real system and hits public boundaries. The skill writes the test, wires it into the right lane, and proves it catches real regressions unless blocked by unavailable infrastructure."
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Create or strengthen an automated E2E for `$ARGUMENTS`, recent changes, or the conversation target. Deliver the test, its lane wiring, and verification. For one-off runtime QA without automated tests, use `e2e-live-check`.

## Discover the real path

Read the effective diff, applicable repo instructions, and the code that establishes the public boundary, startup, dependencies, auth/context, readiness, cleanup, and existing E2E lane. Inspect command implementations where names or docs are ambiguous; a script named “smoke” is not proof of its behavior.

Prefer an existing lane and repo-owned clients that handle auth, routing, headers, and tenant context. Preserve required values, defaults, and intentional omissions, including negative cases. Do not invent deployed headers or tenant identifiers. Use external simulators only for uncontrollable third parties outside the behavior under test, speaking their real protocol.

Use bounded help/version/status calls for discovery. Before starting the runtime, state a short contract: behavior and public boundary, route and decisive sources, required real backends, success and highest-risk negative case, lane, permitted simulators, cleanup, and material unknowns. When the request already authorizes execution, do not add a separate route document or approval checkpoint.

## Implement and run

1. Add the smallest idiomatic test that drives the real public API, UI, CLI, worker trigger, or protocol from outside the engine and observes the product outcome (responses, persisted state, events, files, or other effects).
2. Cover the main scenario and the highest-risk edge or negative case from the changed behavior through the same real boundary. Explain meaningful omissions; do not pursue exhaustive coverage.
3. Wire the test into the existing discoverable local E2E command and CI/test lane, or the smallest adjacent lane if none owns the boundary. Required CI mode must fail when required infrastructure is absent; optional local availability gating must not masquerade as a pass.
4. Start or reach the real engine and required backends. Prove readiness with a probe that covers the selected boundary; process startup or an unrelated healthcheck is insufficient. Track created processes/resources and collect failure artifacts. Use existing authorization and request only missing permissions for the chosen action.
5. Run the exact E2E command and fix in-scope test, wiring, or product defects. Add a guard against fake paths or lost lane wiring only when an existing gap justifies it.
6. Prove the test catches a realistic regression when safe: reuse current same-scope fail/pass evidence, or temporarily break the relevant code/config/fixture in an isolated local checkout and runtime, observe the expected failure, restore, and rerun green. Never inject a regression into a shared deployment. If this proof is unsafe, explain the limitation.
7. Clean up created resources and processes, preserve useful artifacts, inspect the final diff, and report any intentionally retained runtime or resource.

Do not substitute mocks or fake backends inside the tested path, internal helpers, test-only endpoints, bypass flags, broad sleeps, skipped failures, weak snapshots, or “server starts” assertions for E2E evidence.

## Resolve obstacles

Repair in-scope setup problems from logs, readiness, config, and lane wiring. Use `auth-preflight` for missing or expired access and command approval for sandbox restrictions, then resume. A timeout or unavailable client does not prove the capability is unavailable; try a supported equivalent route without changing identity or weakening the contract. Request a necessary human action when discovered and continue independent work while waiting. Stop only the affected path when no safe useful next action remains; keep its failed or unverified status and finish the remaining authorized work.

## Finish

Completion requires a wired test, the real boundary and backends, readiness, passing main and selected edge cases, regression proof or its stated safety limit, and cleanup. Reused proof must cover the current diff, lane, runtime/backends, inputs, and observations.

Report the tested behavior, route and decisive sources, command results, edge coverage and omissions, regression proof, changed files, artifacts, cleanup, and remaining blockers, proportional to the work. Do not end with only a plan or claim behavior that was not observed.
