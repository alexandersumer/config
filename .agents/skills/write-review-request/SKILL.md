---
name: write-review-request
description: Write a clear, concise review request for an existing pull request that a teammate can understand without reading the diff, covering the behavior, reason, relevant evidence, and rollout safety. Use for review requests, not performing reviews, writing PR descriptions, or assigning reviewers.
---

Write the message only. Do not send it, create a pull request, or assign reviewers.

## Ground the message

Use the supplied PR, diff, and relevant conversation evidence. Read the target with read-only tools when needed. Cover the whole PR's purpose, not just its latest edit, and exclude unrelated local work. Ask one focused question only if the target or essential context cannot be resolved.

Never invent motivation, repro results, measurements, test results, flag states, or rollout guarantees. A test in the diff is not a passing test; a reproduction recipe is not an executed repro. Evidence must apply to the version and behavior described. Distinguish local checks, CI, and deployed verification. State a material unknown plainly instead of implying it is verified.

## Make the change understandable

Write for a technically capable teammate who has not followed the implementation. The message should stand on its own: the reviewer should understand the problem and resulting behavior before opening the PR. Clarity takes priority over brevity or filling a template.

- Start with who or what is affected and what happens differently. For internal changes, name the responsibility being moved or shared and the practical reason. Do not force a customer impact onto maintenance work.
- Explain the cause and consequence: under what condition does the problem occur, and what does it prevent or cause? Replace abstract summaries such as "improve lifecycle handling" with the actual behavior, such as "close the connection when the client disconnects."
- Use familiar verbs and explicit subjects. Name what is waiting, failing, being limited, or being stopped. Avoid compressed lists of mechanisms that make readers reconstruct how they fit together.
- Keep technical terms when they identify a relevant component, contract, or review decision. Explain an unfamiliar term through its role or effect on first use; do not replace precise technical meaning with a vague simplification. Keep exact identifiers only when they help locate or assess the change.
- Make before/after comparisons use the same trigger and observable outcome. State what the reproduction demonstrated without generalizing beyond it.
- Ask reviewers to assess a concrete behavior, decision, or failure case. Instead of "check cleanup semantics," say what must be released, preserved, or allowed to finish and when.

## Make it scannable

Open with a short review ask and the PR link, such as `Could someone please review [#<number>](<URL>)?`. Then leave a blank line and use bullets with **bold, descriptive labels**. Each bullet answers one question:

- **What:** What changes for the user or caller? Describe the resulting behavior or concrete refactor.
- **Why:** What problem or need makes this worth doing? Name the impact, not a generic benefit.
- **Repro:** For a bug, give the smallest useful trigger and the observed before/after result. If only suggested steps are available, use **To check** and make clear they have not been run. For a feature, an **Example** may be more useful.
- **Validation:** What was actually checked, where, and with what result? Mention a verification gap when it matters to assessing this change. Keep this separate from Repro when it adds distinct evidence; omit repetition.
- **Rollout:** For behavior changes, explain who is exposed and what controls activation or rollback. If a feature flag is relevant, name it and distinguish its code default from its verified deployed state. Say what disabling it restores only when supported; mention persistent changes or other limits that affect rollback. If rollout details are unknown and affect safety, say what remains unverified. A flag's existence alone does not prove the change is isolated or safe.
- **Review:** Include only when there is a specific decision, tradeoff, or risky boundary where the reviewer's judgment would help. Ask for that judgment directly.

Use these as a default order, not a quota. Always explain What and Why; add the other bullets when they carry useful facts or material gaps. Do not invent a repro or rollout ceremony for a docs change or behavior-preserving refactor. Include a dependency, compatibility constraint, migration, or limitation under the closest label, or give it its own short label when it needs attention.

Aim for roughly 80–140 words, usually three to six bullets; this is a guide, not a limit. Use one short sentence per bullet, or two when needed for a result or caveat. Prefer a slightly longer clear message over squeezing distinct facts into dense sentences. Omit empty sections, nested lists, file inventories, and test-command dumps. Follow an explicitly requested format or length instead of forcing this default.

## Keep the voice natural

Write as a colleague asking for a review. Use concrete subjects and ordinary verbs. Skip "This PR introduces," "ensures," "streamlines," "robust," generic safety claims, fake urgency, em dashes, and closing sales pitches. Keep identifiers only when they help the reviewer, such as the exact feature flag or an affected API.

Before returning, do a silent clarity pass:

- Can a teammate explain the problem and change after one reading, without knowing the PR's internals? Rewrite any sentence that requires decoding a term or inferring a missing causal link.
- Does each bullet add distinct information? Cut repetition, generic benefits, and implementation details that do not help the review.
- Are evidence and material unknowns accurate and easy to distinguish? Include only gaps relevant to this change; do not append a standard list of unrun environments.
- Does the review ask identify what needs judgment, and does the rollout wording describe only supported controls and limits?

Preserve the facts while simplifying the language; do not silently strengthen a guarantee to make the message sound clear. Return only the ready-to-copy message, without a code fence or writing commentary.

## Examples

These are fictional style examples; never reuse their facts or flag names.

Could someone please review [#4328](https://example.com/pull/4328)?

- **What:** Show a payment with an unknown outcome as pending until the payment provider confirms whether it charged the customer.
- **Why:** When the provider times out after accepting a payment, we currently show a failure and invite the customer to pay again.
- **Validation:** A local test simulating that timeout passed: the payment stayed pending until confirmation arrived. Provider sandbox testing is still pending.
- **Rollout:** Enabled for staff accounts only. Turning it off restores the previous display; it does not reverse payments already submitted.
- **Review:** Please check whether the pending message gives customers enough information while we wait for confirmation.

Could someone please review [#1234](https://example.com/pull/1234)?

- **What:** Have all three importers use the same date parser, preserving each importer's accepted formats.
- **Why:** A date-parsing fix currently has to be copied into three places, and those copies have drifted.
- **Validation:** Existing tests for all three importers pass, including their different date formats.
