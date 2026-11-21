---
name: drive-to-completion
description: Take ownership of unfinished engineering work through delivery and ground-truth verification. Use when asked to pick up a session, finish an effort, or drive remaining work through review, CI, and acceptance.
---

# Drive to Completion

1. **Reestablish ground truth.** Check the owning code and spec, current PR heads and reviews, and hosted CI. Check the deployed revision and configuration when acceptance depends on them. Use prior summaries as leads. Start with the highest-impact unfinished work and group related changes around observable acceptance criteria.
2. **Own delivery.** Implement each change, update affected specs and runbooks, resolve actionable review feedback, and watch CI through completion. Diagnose failures, try recoverable paths, and publish or merge where authorized. Keep moving on independent work until the scope is complete or every remaining path has a concrete blocker.
3. **Prove each claim.** Exercise the normal entry point and check the outcome at the boundary users or callers observe. For important regressions, run the same check on the baseline and fix. Show that it fails for the intended reason and passes with the fix. Confirm hosted CI on the exact head SHA and recheck after changes. For live acceptance, record the environment, deployed revision, and observed result. State what local, hosted, and live checks each establish. Mark untested outcomes as unverified.
4. **Close against acceptance criteria.** Report what merged or was delivered, link the evidence, and name each remaining blocker and who or what can clear it. Take any authorized step that can still advance the work before handing it back.
