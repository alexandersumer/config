---
name: diagnose
description: Diagnose hard bugs and performance regressions. Use when the failure is unclear, flaky, not yet reproduced, or needs a disciplined debug loop before fixing.
---

## Validation reuse and check scope

Before running a slow, broad, external, stateful, or CI-equivalent command, check whether this conversation or current-SHA CI/artifacts already contain usable proof. Reuse prior passing evidence instead of rerunning only when it is visible, ran after the last relevant edit, covers the same command/scenario and behavior, edge case, or public boundary, and no touched file, config, dependency, fixture, generated output, runtime state, or environment assumption it depends on changed afterward. If uncertain, run the narrowest freshness check that resolves the uncertainty before escalating.

Default to the narrowest honest proof. Run broader suites, full builds, CI reruns, or live/E2E flows only when required by blast radius, merge/release policy, changed shared API/schema/build/test infrastructure/dependencies/auth/security/persistence/concurrency, merge/conflict integration risk, missing targeted seams, or because the broad command is the only proof that covers the behavior.

Final reports must distinguish reused proof, newly run commands, and checks intentionally not run.

Diagnose `symptom`, `$ARGUMENTS`, pasted error, or the conversation bug.

Do not hypothesize without a feedback loop. First build the smallest deterministic agent-runnable signal that shows the user's failure: test, CLI command, HTTP script, browser script, trace replay, fixture harness, fuzz loop, differential run, or repeated stress loop for flakes. If no loop is possible, stop with what you tried and the artifact/access needed.

Reproduce the real symptom, not a nearby failure. Capture the exact error, output, timing, or state that proves it.

Before changing code, write 3-5 ranked falsifiable hypotheses. Each must predict what observation or one-variable probe would confirm or refute it. Test the highest-signal probe first. Do not log everything; instrument only boundaries that distinguish hypotheses. Tag temporary logs with a unique `[DEBUG-...]` prefix.

For performance regressions, measure first: baseline, profiler/timing/query plan, then bisect or isolate. Do not guess from code shape.

Red flags: stop and return to the loop if you think "probably", "quick fix", "try this", "obvious", "test is wrong", or "clean it up later".

Fix the proven root cause only. Add or preserve a regression check at the seam that exercises the real bug pattern. If no correct seam exists, say the architecture prevents a durable regression check and name the gap.

Before final: rerun or reuse fresh proof for the original loop and regression check under the validation policy, then remove all `[DEBUG-...]` instrumentation and throwaway harnesses.

Final:
- Reproduced: `<signal>`
- Cause: `<proven root cause>`
- Fixed: `<files>`
- Regression: `<check or no correct seam: gap>`
- Checks: `<commands>`
- Remaining: `<none or blocker>`
