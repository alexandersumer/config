---
name: real-e2e
description: "Implement real engine end-to-end tests to completion. Use when the user wants a real E2E, true E2E, smoke test, acceptance test, CI proof, or asks to boot the system and hit public APIs while rejecting mocks/fakes/workarounds. This is an execution skill: it must create or harden the test, wire it into the right lane, run it, and prove it catches real regressions unless blocked by unavailable infrastructure."
---

Create a **real** end-to-end test for `scope`, `$ARGUMENTS`, recent changes, or the conversation target, and keep going until the test exists, is wired into the appropriate lane, and has been validated or is blocked by a concrete infrastructure limitation. Real means the production engine is running and the test drives the same public boundary a user, service, CLI, worker, or protocol client depends on.

The proof must have two parts unless blocked by concrete infrastructure or safety limits:
1. **Automated E2E proof**: a test or smoke lane wired so future CI can run it.
2. **Real-spin proof**: operate the changed system through the repo-documented real run/E2E path, outside the new test's assertions, to confirm the behavior works like it would for a real user/operator/client.

This is not a planning skill. The plan/contract is only a short alignment checkpoint before implementation; it is not an acceptable final output. After writing the contract, immediately do the work: edit files, add the E2E, wire CI/scripts, run the relevant commands, fix failures caused by the new test, and prove the test would catch a real regression when safe.

Gold standard: if available, inspect `~/atlassian/alta-1/tests/e2e-*` and `~/atlassian/alta-1/scripts/e2e-*-smoke.sh` before designing the test. Prefer the patterns that boot Docker/process stacks, wait for readiness, call HTTP/ACP/storage boundaries, persist or observe downstream effects, collect artifacts, and keep policy tests that prevent fake smoke lanes from creeping in.

Contract checkpoint before editing: write the realness contract in chat in no more than a few bullets, then implement. Name the public behavior, top-level API/protocol/CLI/HTTP route, runtime/process/container/backends that must be live, observable success criteria, highest-risk failure mode, CI lane that will run it, allowed out-of-process simulators, and every forbidden shortcut. Include the real-spin contract: which repo docs/scripts/configs describe real E2E operation, the exact manual/operator scenario to run after the automated test, and the small set of relevant edge probes to try. If any of these are unknown, inspect more or ask one focused question, then proceed as far as possible.

Edge-case requirement: before editing, identify a small edge-case matrix for the behavior under test:
- **Automated E2E**: cases covered by the automated test, including at least one highest-risk edge/negative case unless blocked by concrete infrastructure or safety limits.
- **Real-spin probes**: cases to try manually through the documented real run path.
- **Omitted**: meaningful cases intentionally omitted, each with a concrete reason.

Consider applicable invalid or malformed input, empty/minimal input, duplicate/repeated/idempotent calls, missing resources or not-found behavior, permission/auth/config boundaries, persistence across restart or cleanup/retry behavior, concurrency/ordering/timeout/partial-failure risks, and downstream/backend unavailability when safely simulatable outside the engine boundary. Prefer the highest-risk edge over exhaustive coverage. A happy-path-only E2E is incomplete unless the behavior truly has no meaningful edge case and the final answer explains why.

Non-negotiables:
- Hit a public product boundary. Do not import internals, call private helpers, or assert implementation details as the primary proof.
- Boot the real engine path: service, CLI, worker, runtime, container, persistence, queue, cache, object store, or protocol bridge as applicable.
- Use realistic data and verify durable outcomes: response body, file/database/object state, emitted event, downstream side effect, logs, or protocol transcript.
- Use simulators only outside the engine boundary for uncontrollable third-party systems, and name them in the contract. They must speak the real protocol and must not replace the behavior under test.
- Wire the test into an existing E2E/smoke CI step, or add the smallest explicit step adjacent to the matching lane.
- Add availability gating only for missing external infrastructure. Required CI mode must fail when the stack is unavailable.
- Capture cleanup and artifacts for failures without hiding failures.

Forbidden as E2E proof:
- mocks, fake adapters inside the engine path, in-memory replacements for real backends under test, mocked network clients, or edge checks performed only against mocks/fakes instead of the real public boundary;
- test-only endpoints, bypass flags that skip the real path, broad sleeps instead of readiness probes, skipped tests, weakened assertions, or snapshot-only checks;
- “server starts” checks, happy-path-only coverage when meaningful edge cases exist, or tests whose main assertion is that a fake was called.

Implementation loop:
1. Study existing E2E lanes, package scripts, CI config, Docker/process startup, readiness helpers, adjacent tests, and repo-documented real run/E2E instructions such as README files, docs, Makefiles, package scripts, compose files, or smoke scripts. Reuse the closest lane instead of inventing a parallel harness.
2. Add the smallest high-signal test that drives the real boundary from outside the engine and observes the business outcome. The automated E2E must include at least one meaningful edge/negative assertion through the same real public boundary unless blocked by concrete infrastructure or safety limits; choose the highest-risk edge from the matrix rather than broad exhaustive coverage.
3. Wire the test into the existing E2E/smoke command and CI lane, or add the smallest adjacent lane needed for the same purpose.
4. Add or update a policy/guardrail test when needed to prevent future drift back to fakes, internal imports, or untracked CI wiring.
5. Run the exact E2E command and the nearest broader check. Fix failures caused by the new test or wiring. If infrastructure is unavailable locally, run the non-network policy/compile checks and state the missing command as blocked, not passed.
6. Take the changed system for a real spin beyond rerunning tests: boot or invoke the production entrypoint as documented by the repo, drive the public behavior like a user/operator/client would, inspect the durable result or transcript, and try the smallest meaningful set of edge probes for this change, such as invalid input, empty/minimal data, repeated/idempotent calls, cleanup/retry behavior, permission/config boundaries, or persistence across restart when applicable. Fix caused issues found during this spin, then rerun the scenario.
7. Prove the automated E2E would catch a realistic regression when safe: make one temporary reversible break in production code, config, or fixture; confirm the E2E fails for the expected reason; restore; rerun green. If unsafe, explain the exact reason.

Final only after implementation: contract, edge-case matrix, boundary hit, stack/backends booted, CI lane, automated E2E result, automated edge/negative case covered, real-spin scenario and edge probes exercised, regression caught, commands/results, changed files, fixes made from discovered caused issues, edge cases intentionally omitted with reasons, and any remaining blocker.

Never end with only a plan. Never call it real E2E unless it would fail when the real engine behavior is broken and, when infrastructure permits, the changed system has also been operated successfully through its documented real path.
