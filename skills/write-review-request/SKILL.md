---
name: write-review-request
description: Write a clear, concise review request for an existing pull request that a teammate can understand without reading the diff, covering the behavior, reason, repro or example, validation, and rollout safety. Use for review requests, not performing reviews, writing PR descriptions, or assigning reviewers.
---

Write the message only. Do not send it, create a pull request, or assign reviewers.

## Ground the message

Use the supplied PR, diff, and relevant conversation evidence. Read the target with read-only tools when needed. Cover the whole PR's purpose, not just its latest edit, and exclude unrelated local work. Before drafting, look for repro evidence and rollout controls in the supplied material and relevant PR/code context. Ask one focused question only if the target or essential context cannot be resolved; missing repro or rollout evidence should normally be stated in the message.

Never invent motivation, repro results, measurements, test results, flag states, or rollout guarantees. A test in the diff is not a passing test; a reproduction recipe is not an executed repro. Evidence must apply to the version and behavior described. Distinguish local checks, CI, and deployed verification. State a material unknown plainly instead of implying it is verified.

## Make the change understandable

Write for a technically capable teammate who has not followed the implementation. The message should stand on its own: the reviewer should understand the problem and resulting behavior before opening the PR. Clarity takes priority over brevity or filling a template.

- Start with who or what is affected and what happens differently. For internal changes, name the responsibility being moved or shared and the practical reason. Do not force a customer impact onto maintenance work.
- Explain the cause and consequence: under what condition does the problem occur, and what does it prevent or cause? Replace abstract summaries such as "improve lifecycle handling" with the actual behavior, such as "close the connection when the client disconnects."
- Use familiar verbs and explicit subjects. Name what is waiting, failing, being limited, or being stopped. Avoid compressed lists of mechanisms that make readers reconstruct how they fit together.
- Keep technical terms when they identify a relevant component, contract, or review decision. Explain an unfamiliar term through its role or effect on first use; do not replace precise technical meaning with a vague simplification. Keep exact identifiers only when they help locate or assess the change.
- When asking for specific reviewer judgment, name a concrete behavior, decision, or failure case. Instead of "check cleanup semantics," say what must be released, preserved, or allowed to finish and when.

## Make it scannable

Open with a short review ask and the PR link, such as `Could someone please review [#<number>](<URL>)?`. Then leave a blank line and use bullets with **bold, descriptive labels** in this default order:

- **What:** What changes for the user or caller? Describe the resulting behavior or concrete refactor.
- **Why:** What problem or need makes this worth doing? Name the impact, not a generic benefit.
- **Repro:** Give the smallest useful trigger, the environment, and the before/after outcomes supported by evidence. Compare the same trigger and observable outcome. If steps have not been run, label them as suggested and separate expected behavior from observed results. If no usable steps or results are available, say so. A passing test may supply repro evidence when it demonstrates the trigger and outcomes; a generic test summary cannot replace this explanation. For a feature or another behavior change, use **Example** to show the trigger and resulting behavior, identifying whether it is expected or verified.
- **Validation:** State what was actually checked, where, and with what result. Include this bullet when it adds evidence beyond Repro; mention a material verification gap without listing unrelated unrun environments.
- **Rollout:** Explain who or what is exposed, what activates the change, and how it can be disabled or rolled back. Name relevant flags and distinguish code defaults from verified deployed state. Check whether all changed behavior is gated; a flag's existence does not establish isolation. Describe rollback limits, such as persistent data changes, compatibility requirements, or external effects. If exposure, activation, or rollback is unverified, name the missing facts instead of claiming safety.
- **Review:** Include a direct question only when there is a specific decision, tradeoff, or risky boundary where the reviewer's judgment would help.

Always include What and Why. Bug fixes also require Repro and Rollout; other behavior changes require Example (or Repro where applicable) and Rollout. Missing evidence is a reason to state the gap, never to omit a required bullet. Only docs-only changes and behavior-preserving refactors may omit Repro/Example and Rollout; assess the whole PR before applying that exception. Put a dependency, migration, or compatibility constraint under the closest label, or give it a short label of its own when needed.

Aim for roughly 80–140 words, usually three to six bullets. Length and bullet count are guides: never cut required repro/example or rollout information to meet them. Use one or two short sentences per bullet, allowing more space when distinct facts need it. Omit empty sections, nested lists, file inventories, and test-command dumps. If the user requests another format or length, adapt the presentation while retaining the required information unless they explicitly exclude it.

## Keep the voice natural

Write as a colleague asking for a review. Use concrete subjects and ordinary verbs. Skip "This PR introduces," "ensures," "streamlines," "robust," generic safety claims, fake urgency, em dashes, and closing sales pitches. Keep identifiers only when they help the reviewer, such as the exact feature flag or an affected API.

Before returning, do a silent clarity pass:

- Can a teammate explain the problem and change after one reading, without knowing the PR's internals? Rewrite any sentence that requires decoding a term or inferring a missing causal link.
- Does each bullet add distinct information? Cut repetition, generic benefits, and implementation details that do not help the review.
- Are evidence and material unknowns accurate and easy to distinguish? Include only gaps relevant to this change; do not append a standard list of unrun environments.
- For a bug fix, are both Repro and Rollout present? For another behavior change, are Example (or Repro) and Rollout present? Restore any missing required bullet, stating unknowns when evidence is unavailable.
- Does Repro distinguish executed results from proposed steps, and does Rollout cover exposure, activation, rollback, and relevant limits using only supported facts?
- If there is a specific review question, does it identify what needs judgment?

Preserve the facts while simplifying the language; do not silently strengthen a guarantee to make the message sound clear. Return only the ready-to-copy message, without a code fence or writing commentary.

## Examples

These are fictional style examples; never reuse their facts or flag names.

Could someone please review [#4328](https://example.com/pull/4328)?

- **What:** Show a payment with an unknown outcome as pending until the payment provider confirms whether it charged the customer.
- **Why:** When the provider times out after accepting a payment, we currently show a failure and invite the customer to pay again.
- **Repro:** A local simulation accepted the payment but timed out before responding. The same trigger showed failure before the fix and pending until confirmation after it.
- **Validation:** The local regression test passed. Provider sandbox testing is still pending.
- **Rollout:** The `payment-pending` flag gates the display change; its deployed configuration was verified as staff-only. Disabling it restores the previous display but does not reverse submitted payments.
- **Review:** Please check whether the pending message gives customers enough information while we wait for confirmation.

Could someone please review [#1234](https://example.com/pull/1234)?

- **What:** Have all three importers use the same date parser, preserving each importer's accepted formats.
- **Why:** A date-parsing fix currently has to be copied into three places, and those copies have drifted.
- **Validation:** Existing tests for all three importers pass, including their different date formats.

Could someone please review [#5678](https://example.com/pull/5678)?

- **What:** Release the export slot when a client disconnects during an export.
- **Why:** Disconnected exports can hold every slot and prevent new exports from starting.
- **Repro:** Suggested check, not run: disconnect during an export, then start another; the slot should become available. Before/after runtime results are unverified.
- **Rollout:** Exposure, activation controls, and rollback behavior have not been verified; we cannot yet say who receives the change or how to disable it safely.
