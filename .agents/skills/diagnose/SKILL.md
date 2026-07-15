---
name: diagnose
description: Diagnose hard bugs and performance regressions. Use when the failure is unclear, flaky, not yet reproduced, or needs a disciplined debug loop before fixing.
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Diagnose `symptom`, `$ARGUMENTS`, pasted error, or the conversation bug.

Do not hypothesize without falsifiable evidence. Prefer the smallest deterministic agent-runnable signal that shows the user's failure: test, CLI command, HTTP script, browser script, trace replay, fixture harness, fuzz loop, differential run, or repeated stress loop for flakes. When reproduction is unsafe or impossible, continue only if static evidence, historical traces, or captured state can distinguish the ranked hypotheses; otherwise stop with what you tried and the artifact or access needed.

Confirm the real symptom through reproduction or captured evidence, not a nearby failure. Capture the exact error, output, timing, or state that proves it.

Before changing code, write 3-5 ranked falsifiable hypotheses. Each must predict what observation or one-variable probe would confirm or refute it. Test the highest-signal probe first. Do not log everything; instrument only boundaries that distinguish hypotheses. Tag temporary logs with a unique `[DEBUG-...]` prefix.

For performance regressions, measure first: baseline, profiler/timing/query plan, then bisect or isolate. Do not guess from code shape.

Red flags: stop and return to the loop if you think "probably", "quick fix", "try this", "obvious", "test is wrong", or "clean it up later".

Fix the proven root cause only. Add or preserve a regression check at the seam that exercises the real bug pattern, unless stronger existing coverage already catches it. If no correct seam exists, say the architecture prevents a durable regression check and name the gap.

Before final: rerun the original loop or revalidate the captured evidence, and rerun or reuse fresh proof for the regression check under the proof policy. Then remove all `[DEBUG-...]` instrumentation and throwaway harnesses.

Final:
- Signal: `<reproduction or captured evidence>`
- Cause: `<proven root cause>`
- Fixed: `<files>`
- Regression: `<check or no correct seam: gap>`
- Checks: `<commands>`
- Remaining: `<none or blocker>`
