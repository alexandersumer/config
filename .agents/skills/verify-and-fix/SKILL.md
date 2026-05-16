---
name: verify-and-fix
description: Verify behavior with acceptance-level proof and fix only proven defects. Use when the user asks to check, validate, audit, verify, confirm, reproduce, prove, or end-to-end test behavior, especially when they want certainty that a change works rather than a routine CI check.
register_cmd: true
---

Verify `scope`, `$ARGUMENTS`, recent changes, relevant artifact, or conversation behavior by proving the user-visible contract through the real product path.

Do not rubber-stamp. **Verified** and **Fixed** require source proof plus acceptance-level executable proof from this turn. Vanilla CI, typechecks, linters, broad green suites, mocked unit tests, and helper-level assertions are supporting evidence only unless they exercise the real public contract.

Mental model: be a skeptical release owner. Try to make the feature or fix fail in the most realistic available environment. Prefer running the app, service, or tool and observing behavior at its public boundary. Spin up dependencies, seed data, call APIs, use the CLI/UI, inspect persisted state, and check logs/events when those are part of the contract. If the real path cannot be exercised, say **not proven** or **blocked**.

Follow this loop:

1. **Define the acceptance contract.** Use primary sources only: user request, spec, public docs, schemas, types, intended-behavior tests, callers, migrations, or existing product behavior. Convert them into observable criteria: inputs, actions, outputs, state changes, side effects, errors, and non-goals. If the contract is undefined, stop with **not proven**.

2. **Name meaningful failure modes.** State the realistic defects that would violate the contract and where they should be visible: UI, CLI, API response, database/file state, emitted event, downstream call, logs, permissions, or integration boundary.

3. **Trace the real path.** Read control, data, and error flow from public entry point to observable outcome. Include config, flags, routing, serialization, persistence, auth, async work, retries, and integrations when relevant. Mocks, snapshots, private helpers, and implementation-detail assertions are not proof.

4. **Build the deepest practical harness.** Prefer:
   - real end-to-end flow with the app/service/tool running and real or faithful local dependencies;
   - public-boundary integration/API/CLI/browser flow;
   - lower-level reproduction only when no executable public seam exists.

   Make setup reproducible: note seed data, commands, URLs, fixtures, credentials assumptions, and cleanup. Do not stop at CI just because it is green.

5. **Run acceptance proof.** Exercise the real path. Check actual externally visible outcomes, not just exit status. Cover the happy path and the highest-risk negative/edge path needed to rule out the named failure modes.

6. **Fix only proven gaps.** If proof exposes a real defect or acceptance-coverage gap, make the smallest root-cause change. Do not use suppressions, baselines, skipped/weakened tests, sleeps, broad catches, silent fallbacks, or mock-only fixes.

7. **Prove the fix hard.** Rerun the same acceptance harness and show the before/after distinction when available. If adding an automated check, prove when feasible that it fails for the realistic regression, restore, then rerun green. Run broader relevant regression checks after acceptance proof passes.

8. **Escalate uncertainty.** If access, environment, dependencies, data, credentials, or observable seams are missing, do not speculate. Return **not proven** or **blocked** and name the exact missing proof.

Verdicts:
- **Verified** — source proof and acceptance proof support the contract through the real public path; no changes needed.
- **Fixed** — defect or coverage gap was proven, fixed at root cause, and rerun through acceptance proof.
- **Not proven** — contract, source proof, acceptance proof, realistic environment, or public seam is missing.
- **Blocked** — required access, artifact, dependency, credential, environment, or external system is unavailable.

Final:
- Verdict: `<verified | fixed | not proven | blocked>`
- Contract: `<source and observable acceptance criteria>`
- Failure modes: `<realistic defects checked>`
- Source proof: `<entry points/files/functions/config/flow>`
- Acceptance proof: `<environment/setup/actions/observations/results or missing proof>`
- Regression proof: `<targeted/broader commands and results, or why not run>`
- Changed: `<files or none>`
- Remaining: `<none or exact gap/blocker>`
