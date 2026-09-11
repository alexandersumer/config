---
name: e2e-live-check
description: "Operate a real running system end to end without writing automated tests or running CI/test lanes. Use when the user wants live E2E QA of the current effective diff through local real services, agent protocol clients, CLI, browser, HTTP APIs, staging/dev shards, or deployed resources. The skill discovers the most appropriate real-enough environment, drives the public boundary, covers relevant edge cases, captures evidence, cleans up, and reports blockers honestly."
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Operate `scope`, `$ARGUMENTS`, or the current effective diff through a real running system. Produce runtime evidence without writing automated tests or running CI/test lanes. For durable automated coverage, use `e2e-automated-tests`.

## Choose the route

Read the effective diff, applicable repo instructions, and the code, scripts, runbooks, or tests that establish the public boundary, startup, dependencies, auth/context, readiness, and cleanup. Tests and CI files may explain the runtime but are not the live-check proof. Verify ambiguous script names or stale docs against their implementation.

Use the client surface that matches the behavior: browser for UI, CLI for operator commands, protocol client for protocol behavior, or SDK/HTTP for the product API. Prefer repo-owned clients when they handle auth, routing, headers, service proxies, or tenant context. Preserve required values, defaults, and intentional omissions; never invent deployed headers or identifiers.

Prefer an already-running correct local system, then a repo-supported local real stack. Use dev/staging when local cannot reproduce the relevant deployed behavior, such as auth, networking, image wiring, or scheduling. Production access requires explicit authorization already present in the task or obtained for the exact action. Shared-resource mutations need a bounded target and cleanup or rollback; avoid touching unrelated resources.

During discovery, use bounded help/version/status calls. Before starting the selected route, briefly state the behavior, public boundary, environment and decisive sources, expected outcome, highest-risk edge probe, and cleanup. Include material unknowns or missing authorization. This is not a second approval step for work already authorized.

## Operate and verify

1. Start or connect using the verified route. Track process IDs, ports, URLs, created resources, and log locations needed for cleanup. Confirm tools, auth, and the intended boundary are reachable.
2. Prove readiness with a relevant probe. A process start or unrelated healthcheck does not prove the changed path is ready; avoid broad sleeps.
3. Drive the main user/operator scenario through the public boundary and capture its observable outcome: response, file/database/object state, emitted event, UI snapshot, log, or protocol transcript.
4. Exercise the highest-risk meaningful edge or negative case through that same boundary when safe. Consider changed input validation, not-found behavior, auth/config boundaries, repeated calls, persistence, routing, concurrency, timeout, retries, or partial failures. Explain meaningful omissions rather than claiming exhaustive coverage.
5. Check post-operation health or invariants when the flow leaves state behind. If the check exposes an in-scope product defect and a fix is authorized, fix it and rerun the affected scenario and edge probe. Otherwise report the defect with evidence.
6. Clean up resources and processes you created. Preserve useful artifacts and report anything intentionally left running. Do not stop pre-existing user services as cleanup.

Reuse evidence only under the proof policy for the same current diff, public boundary, runtime/resource, inputs, state, and expected observations. When liveness matters, run the cheapest current readiness/status probe. Reused full-flow evidence also needs cleanup evidence or an account of remaining state.

Do not replace the changed path with mocks, fake adapters, internal helpers, test-only endpoints, bypass flags, unauthenticated shortcuts, or a test-lane run. Do not substitute local-only evidence for a contract requiring deployed behavior.

## Resolve obstacles

If the first route fails, inspect relevant logs, readiness, config loading, and documented auth/setup paths. Fix in-scope setup problems or switch to another faithful route. Refresh expired access through the documented workflow. Ask only for a missing fact or permission that can unlock progress; stop when no safe useful action remains. Report the exact unavailable boundary or environment, not a guessed pass.

## Finish

Completion requires the real boundary and environment, readiness, observed main and selected edge outcomes, applicable post-operation checks, and cleanup. Report the route and decisive sources, commands and outcomes, new or reused evidence, edge omissions, artifacts, fixes, cleanup, and exact remaining blockers. Keep the report proportional to the work and limit claims to the checked behavior.
