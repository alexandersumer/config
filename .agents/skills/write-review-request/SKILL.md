---
name: write-review-request
description: Write a concise, structured message asking someone to review an existing pull request, covering the change, reason, evidence, and rollout safety. Use for review requests, not performing reviews, writing PR descriptions, or assigning reviewers.
---

Write the message only. Do not send it, create a pull request, or assign reviewers.

## Ground the message

Use the supplied PR, diff, and relevant conversation evidence. Read the target with read-only tools when needed. Cover the whole PR's purpose, not just its latest edit, and exclude unrelated local work. Ask one focused question only if the target or essential context cannot be resolved.

Never invent motivation, repro results, measurements, test results, flag states, or rollout guarantees. A test in the diff is not a passing test; a reproduction recipe is not an executed repro. Evidence must apply to the version and behavior described. Distinguish local checks, CI, and deployed verification. State a material unknown plainly instead of implying it is verified.

## Make it scannable

Open with a short review ask and the PR link, such as `Could someone please review [#<number>](<URL>)?`. Then leave a blank line and use bullets with **bold, descriptive labels**. Each bullet answers one question:

- **What:** What changes for the user or caller? Describe the resulting behavior or concrete refactor.
- **Why:** What problem or need makes this worth doing? Name the impact, not a generic benefit.
- **Repro:** For a bug, give the smallest useful trigger and the observed before/after result. If only suggested steps are available, use **To check** and make clear they have not been run. For a feature, an **Example** may be more useful.
- **Validation:** What was actually checked, where, and with what result? Include a material gap. Keep this separate from Repro when it adds distinct evidence; omit repetition.
- **Rollout:** For behavior changes, explain who is exposed and what controls activation or rollback. If a feature flag is relevant, name it and distinguish its code default from its verified deployed state. Say what disabling it restores only when supported; mention persistent changes or other limits that affect rollback. If rollout details are unknown and affect safety, say what remains unverified. A flag's existence alone does not prove the change is isolated or safe.
- **Review:** Include only when there is a specific decision, tradeoff, or risky boundary where the reviewer's judgment would help. Ask for that judgment directly.

Use these as a default order, not a quota. Always explain What and Why; add the other bullets when they carry useful facts or material gaps. Do not invent a repro or rollout ceremony for a docs change or behavior-preserving refactor. Include a dependency, compatibility constraint, migration, or limitation under the closest label, or give it its own short label when it needs attention.

Aim for roughly 80–140 words, usually three to six bullets. Use one short sentence per bullet, or two when needed for a result or caveat. Prefer a slightly longer clear message over squeezing distinct facts into dense sentences. Omit empty sections, nested lists, file inventories, and test-command dumps. Follow an explicitly requested format or length instead of forcing this default.

## Keep the voice natural

Write as a colleague asking for a review. Use concrete subjects and ordinary verbs. Skip "This PR introduces," "ensures," "streamlines," "robust," generic safety claims, fake urgency, em dashes, and closing sales pitches. Keep identifiers only when they help the reviewer, such as the exact feature flag or an affected API.

Before returning, read it as a reviewer: can they quickly tell what changes, why, what evidence exists, and how exposure is controlled? Cut repetition and any sentence that could describe almost any PR. Return only the ready-to-copy message, without a code fence or writing commentary.

## Examples

These are fictional style examples; never reuse their facts or flag names.

Could someone please review [#4328](https://example.com/pull/4328)?

- **What:** Enforce the deadline while an output read is blocked.
- **Why:** A stalled read can leave the caller waiting past its timeout.
- **Repro:** Pause Valkey for 3 seconds with a 250 ms deadline. Before: late completion; after: timeout.
- **Validation:** Reproduced against real Valkey locally; staging is untested.
- **Rollout:** Behind `blocked_read_deadline`, default off in code; deployed flag state is unverified. Disabling it restores the old read path.
- **Review:** The command still runs after output collection stops; please check that this matches the cancellation contract.

Could someone please review [#1234](https://example.com/pull/1234)?

- **What:** Share one deadline handler across three callers, with no intended behavior change.
- **Why:** Timeout fixes currently need to be repeated in three places.
- **Validation:** Existing timeout and cancellation tests pass for all three callers.
