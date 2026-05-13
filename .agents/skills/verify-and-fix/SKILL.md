---
name: verify-and-fix
description: Verify behavior and fix proven defects. Use when the user asks to check, validate, audit, or confirm behavior and repair only real gaps.
register_cmd: true
---

Verify `scope`, `$ARGUMENTS`, recent changes, relevant artifact, or conversation behavior.

Do not rubber-stamp. A verdict needs both source proof and executable proof from this turn. If either is missing, say **not proven** and name the missing artifact, path, command, environment, or seam.

Follow this loop:

1. **State the contract.** Derive expected behavior from primary sources only: user request, artifact/spec, public API docs, schemas, types, intended-behavior tests, callers, migrations. If no source defines the behavior, stop with **not proven**.

2. **Name the bug that would matter.** Before running checks or editing, say what realistic defect would violate the contract and where it should be caught.

3. **Trace source proof.** Read the real control, data, and error flow through the public path. Do not treat mocks, private helpers, or implementation-detail assertions as proof.

4. **Run executable proof.** Use the smallest command, test, script, fixture, API call, or reproduction that exercises the real path. Broad green tests alone are not proof unless they exercise the named contract.

5. **Fix only after proof fails.** If source proof or executable proof exposes a real defect or verification gap, make the smallest code/test change at the root cause. No suppressions, baselines, dependency bumps, skipped/deleted/weakened tests, sleeps, broad catches, or silent fallbacks.

6. **Prove the fix.** Rerun the targeted proof. If you added or changed a check, prove when feasible that it would fail for the realistic regression, then restore and rerun green. Run the broader relevant suite after targeted proof is green.

Verdict meanings:
- **Verified** — source proof and executable proof both support the contract; no changes needed.
- **Fixed** — a real defect or verification gap was proven, changed, and rerun green.
- **Not proven** — expected behavior, source proof, executable proof, or a checkable seam is missing.
- **Blocked** — required access, artifact, dependency, or environment is unavailable.

Final:
- Verdict: `<verified | fixed | not proven | blocked>`
- Contract: `<expected behavior and source>`
- Bug model: `<realistic defect checked>`
- Source proof: `<files/functions/flow>`
- Executable proof: `<commands/results or missing proof>`
- Changed: `<files or none>`
- Remaining: `<none or gap/blocker>`
