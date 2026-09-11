---
name: e2e-automated-tests
description: "Create or harden real automated end-to-end tests. Use when the user wants durable E2E regression coverage, smoke or acceptance tests, CI/test-lane proof, or an automated test that boots the real system and hits public boundaries. The skill writes the test, wires it into the right lane, and proves it catches real regressions unless blocked by unavailable infrastructure."
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Create or strengthen an automated E2E for `scope`, `$ARGUMENTS`, recent changes, or the conversation target. Deliver the test, its lane wiring, and verification. For one-off runtime QA without automated tests, use `e2e-live-check`.

## Discover the real path

Read the effective diff, applicable repo instructions, and the scripts, tests, or runtime code that establish the public boundary, startup, dependencies, auth/context, readiness, cleanup, and existing E2E lane. Inspect command implementations where names or docs are ambiguous; a script named “smoke” is not proof of its behavior. Read only enough to choose a faithful route.

Prefer an existing lane and repo-owned clients that handle auth, routing, headers, and tenant context. Preserve required values, defaults, and intentional omissions, including negative cases. Do not invent deployed headers or tenant identifiers. Use external simulators only for uncontrollable third parties outside the behavior under test, speaking their real protocol.

Use bounded help/version/status calls for discovery. Before starting the selected runtime, state a short contract: behavior and public boundary, route and decisive sources, required real backends, success and highest-risk negative case, lane, permitted simulators, and cleanup. Include material unknowns; do not produce a separate route document or approval checkpoint when the request already authorizes execution.

## Implement and run

1. Add the smallest idiomatic test that drives the real public API, UI, CLI, worker trigger, or protocol from outside the engine and observes the product outcome. Verify responses, persisted state, events, files, or other effects appropriate to that behavior.
2. Cover the main scenario and the highest-risk meaningful edge or negative case through the same real boundary. Select from the changed behavior: invalid or missing inputs, repeated calls, permissions, persistence, concurrency, ordering, partial failure, or unavailable dependencies. Explain meaningful omissions; do not pursue exhaustive coverage.
3. Wire the test into the existing discoverable local E2E command and CI/test lane. If none owns the boundary, add the smallest adjacent lane. Required CI mode must fail when required infrastructure is absent; optional local availability gating must not masquerade as a pass.
4. Start or reach the real engine and required backends. Prove readiness with a probe that covers the selected boundary; process startup or an unrelated healthcheck is insufficient. Track created processes/resources and collect useful failure artifacts. Use existing authorization and request only missing permissions for the chosen action.
5. Run the exact E2E command, diagnose failures, and fix in-scope test, wiring, or product defects. Broaden checks only under the proof policy. Add a guard against fake paths or lost lane wiring only when an existing gap justifies it.
6. Prove the test catches a realistic regression when safe. Reuse current same-scope fail/pass evidence, or temporarily break the relevant code/config/fixture in an isolated local checkout and runtime, observe the expected failure, restore, and rerun green. Never inject a regression into a shared deployment. If this proof is unsafe, explain the limitation.
7. Clean up resources and processes created for the run, preserve useful artifacts, and inspect the final diff. Report any intentionally retained runtime or resource.

Do not substitute mocks or fake backends inside the tested path, internal helpers, test-only endpoints, bypass flags, broad sleeps, skipped failures, weak snapshots, or “server starts” assertions for E2E evidence.

## Resolve obstacles

If the first route fails, inspect the relevant logs, readiness, config loading, auth path, and lane wiring. Try another repo-supported route only if it tests the same contract. Refresh expired access through the documented auth workflow and fix local setup within scope. Ask only for a missing fact or authorization that would unlock progress. Stop with the exact blocker when no safe useful action remains; do not exhaust unrelated setup paths or quietly weaken the contract.

## Finish

Completion requires a wired test, the real boundary and backends, readiness, passing main and selected edge cases, regression proof or its stated safety limit, and cleanup. Reused proof must cover the current diff, lane, runtime/backends, inputs, and observations.

Report the tested behavior, selected route and decisive sources, command results, edge coverage and omissions, regression proof, changed files, artifacts, cleanup, and remaining blockers. Keep the report proportional to the work. Do not end with only a plan or claim behavior that was not observed.
