---
name: real-e2e
description: "Implement real engine end-to-end tests to completion. Use when the user wants a real E2E, true E2E, smoke test, acceptance test, CI proof, or asks to boot the system and hit public APIs while rejecting mocks/fakes/workarounds. This is an execution skill: it must create or harden the test, wire it into the right lane, run it, and prove it catches real regressions unless blocked by unavailable infrastructure."
register_cmd: true
---

Create a **real** end-to-end test for `scope`, `$ARGUMENTS`, recent changes, or the conversation target, and keep going until the test exists, is wired into the appropriate lane, and has been validated or is blocked by a concrete infrastructure limitation. Real means the production engine is running and the test drives the same public boundary a user, service, CLI, worker, or protocol client depends on.

This is not a planning skill. The plan/contract is only a short alignment checkpoint before implementation; it is not an acceptable final output. After writing the contract, immediately do the work: edit files, add the E2E, wire CI/scripts, run the relevant commands, fix failures caused by the new test, and prove the test would catch a real regression when safe.

Gold standard: if available, inspect `~/atlassian/alta-1/tests/e2e-*` and `~/atlassian/alta-1/scripts/e2e-*-smoke.sh` before designing the test. Prefer the patterns that boot Docker/process stacks, wait for readiness, call HTTP/ACP/storage boundaries, persist or observe downstream effects, collect artifacts, and keep policy tests that prevent fake smoke lanes from creeping in.

Contract checkpoint before editing: write the realness contract in chat in no more than a few bullets, then implement. Name the public behavior, top-level API/protocol/CLI/HTTP route, runtime/process/container/backends that must be live, observable success criteria, highest-risk failure mode, CI lane that will run it, allowed out-of-process simulators, and every forbidden shortcut. If any of these are unknown, inspect more or ask one focused question, then proceed as far as possible.

Non-negotiables:
- Hit a public product boundary. Do not import internals, call private helpers, or assert implementation details as the primary proof.
- Boot the real engine path: service, CLI, worker, runtime, container, persistence, queue, cache, object store, or protocol bridge as applicable.
- Use realistic data and verify durable outcomes: response body, file/database/object state, emitted event, downstream side effect, logs, or protocol transcript.
- Use simulators only outside the engine boundary for uncontrollable third-party systems, and name them in the contract. They must speak the real protocol and must not replace the behavior under test.
- Wire the test into an existing E2E/smoke CI step, or add the smallest explicit step adjacent to the matching lane.
- Add availability gating only for missing external infrastructure. Required CI mode must fail when the stack is unavailable.
- Capture cleanup and artifacts for failures without hiding failures.

Forbidden as E2E proof: mocks, fake adapters inside the engine path, in-memory replacements for real backends under test, test-only endpoints, snapshot-only checks, “server starts” checks, mocked network clients, bypass flags that skip the real path, broad sleeps instead of readiness probes, skipped tests, weakened assertions, or tests whose main assertion is that a fake was called.

Implementation loop:
1. Study existing E2E lanes, package scripts, CI config, Docker/process startup, readiness helpers, and adjacent tests. Reuse the closest lane instead of inventing a parallel harness.
2. Add the smallest high-signal test that drives the real boundary from outside the engine and observes the business outcome. Include at least one failure/edge assertion when that is the risk being protected.
3. Wire the test into the existing E2E/smoke command and CI lane, or add the smallest adjacent lane needed for the same purpose.
4. Add or update a policy/guardrail test when needed to prevent future drift back to fakes, internal imports, or untracked CI wiring.
5. Run the exact E2E command and the nearest broader check. Fix failures caused by the new test or wiring. If infrastructure is unavailable locally, run the non-network policy/compile checks and state the missing command as blocked, not passed.
6. Prove the test would catch a realistic regression when safe: make one temporary reversible break in production code, config, or fixture; confirm the E2E fails for the expected reason; restore; rerun green. If unsafe, explain the exact reason.

Final only after implementation: contract, boundary hit, stack/backends booted, CI lane, regression caught, commands/results, changed files, and any remaining blocker. Never end with only a plan. Never call it real E2E unless it would fail when the real engine behavior is broken.
