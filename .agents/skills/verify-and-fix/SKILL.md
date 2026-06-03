---
name: verify-and-fix
description: Verify behavior with acceptance-level proof and fix only proven defects. Use when the user asks to check, validate, audit, verify, confirm, reproduce, prove, or end-to-end test behavior, especially when they want certainty that a change works rather than a routine CI check.
---

Verify `scope`, `$ARGUMENTS`, recent changes, relevant artifact, or conversation behavior by proving the user-visible contract through the real product path.

Standard: no rubber stamps. **Verified** and **Fixed** require source proof plus acceptance proof from this turn. CI, typechecks, linters, broad green suites, mocked unit tests, snapshots, and helper-level assertions are supporting evidence only. They do not prove the verdict unless they exercise the same public contract and observable outcome a user depends on.

Mental model: be a skeptical release owner. Try to make the feature or fix fail in the most realistic runnable environment. Start by attempting the actual product path: run the app, service, CLI, worker, or tool; use real or faithful local dependencies; seed realistic data; call the API/UI/CLI; inspect persisted state, emitted events, downstream effects, logs, and errors when relevant. Do not choose a smaller proof because it is quicker.

Proof ladder: use the first feasible level, and explicitly justify any downgrade.
1. Real end-to-end flow with the product running and dependencies available.
2. Public-boundary flow: API, CLI, browser/UI, worker, or integration exercising the same contract.
3. Lower-level reproduction only when no executable public seam exists; this cannot support **Verified** or **Fixed** unless it observes the same contract outcome.
4. If none is feasible, return **not proven** or **blocked**.

Follow this loop:

1. **Define the acceptance contract.** Use primary sources only: user request, spec, public docs, schemas, types, intended-behavior tests, callers, migrations, or existing product behavior. Convert them into observable criteria: inputs, actions, outputs, state changes, side effects, errors, and non-goals. If the contract is undefined, stop with **not proven**.

2. **Name meaningful failure modes.** State the realistic defects that would violate the contract and where they should be visible: UI, CLI, API response, database/file state, emitted event, downstream call, logs, permissions, or integration boundary.

3. **Trace the real path.** Read control, data, and error flow from public entry point to observable outcome. Include config, flags, routing, serialization, persistence, auth, async work, retries, and integrations when relevant. Mocks and implementation-detail assertions are not proof.

4. **Build and run acceptance proof.** Use the proof ladder. Make setup reproducible: note commands, URLs, seed data, fixtures, credentials assumptions, and cleanup. Record expected vs actual observations. Check externally visible outcomes, not just exit status. Cover the happy path and the highest-risk negative or edge path needed to rule out the named failure modes.

5. **Fix only proven gaps.** If proof exposes a real defect or acceptance-coverage gap, make the smallest root-cause change. Do not use suppressions, baselines, skipped/weakened tests, sleeps, broad catches, silent fallbacks, or mock-only fixes.

6. **Prove the fix hard.** Rerun the same acceptance proof and show the before/after distinction when available. If adding an automated check, prove when feasible that it fails for the realistic regression, restore, then rerun green. Run broader regression checks only after acceptance proof passes.

7. **Escalate uncertainty.** If access, environment, dependencies, data, credentials, or observable seams are missing, do not speculate. Return **not proven** or **blocked** and name the exact missing proof.

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
