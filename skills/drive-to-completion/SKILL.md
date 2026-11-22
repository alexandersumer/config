---
name: drive-to-completion
description: Take ownership of unfinished engineering work and keep going through delivery and verification. Use when asked to pick up a session, finish an effort, or drive remaining work through review, CI, or acceptance.
---

# Drive to Completion

Stay on the task until the requested outcome is verified, the user asks you to stop, or every remaining item is blocked. Before ending the turn, ask whether an authorized action can still materially advance the outcome. If it can, take it now. A plan, progress update, partial fix, or pending check is an intermediate result.

1. **Establish what remains.** Turn the requested outcome into observable acceptance criteria. Verify prior claims against the owning code, spec, and current delivery state. Check reviews, hosted CI, and the deployed revision where they matter. Start with the highest-impact unfinished criterion.
2. **Advance the outcome.** Act on that criterion in the current turn. Implement or fix it, update affected specs and runbooks, resolve actionable feedback, and publish or merge within the user's authorization. When a check is pending, keep working on independent items, then wait and recheck. When an attempt fails, diagnose the failure and try a safe recovery or another in-scope route. Reassess the remaining criteria after each result, including after resuming.
3. **Verify at the boundary.** Exercise the normal entry point and observe what users or callers see. For important regressions, show the same check failing for the intended reason on the baseline and passing with the fix. Verify hosted checks on the delivered commit and recheck after changes. For live acceptance, record the environment, deployed revision, and observed result.
4. **Close against the criteria.** Report what shipped and link the evidence. For each unmet criterion, mark what remains unverified and give the concrete blocker and the input, permission, or external change needed to clear it. Check for an independent action you can still take before handing back.
