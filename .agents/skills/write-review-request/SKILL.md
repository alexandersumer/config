---
name: write-review-request
description: Write a short, plain message asking someone to review an existing pull request. Use for review-request messages, not performing reviews, writing pull request descriptions, or requesting reviewers through a provider.
---

Write the message only. Do not send it, create a pull request, or assign reviewers.

## Establish the facts

Use the supplied pull request, diff, and relevant conversation evidence. Read the target with read-only tools when needed. Cover its main purpose, not just the latest edit, and exclude unrelated local work. If the target or essential context is missing, ask one focused question.

Never invent motivation, repros, measurements, test results, rollout status, or unchanged behavior. A test in the diff does not mean it passed. Verification must apply to the version and behavior described. Mention an unverified claim as an expectation only when needed; otherwise leave it out.

## Output

Start with `Could someone please review [#<number>](<URL>)?`, then a blank line and three bullets in this order:

1. The problem or reason: what breaks, is missing, or makes the current code difficult to change.
2. The change: what this does to address that problem.
3. The evidence: the smallest useful repro and result, example, measurement, or relevant check. State a material verification gap here if there is one.

Use one short sentence per bullet by default. A second sentence is fine when it adds an essential fact. Omit the evidence bullet if there is no useful evidence or material gap to report. Keep the order; do not add headings, labels such as "Problem:" or "Validation:", nested bullets, or a closing pitch.

Aim for 50–90 words for the whole message; stay under 110 unless the user requests more detail. Shorter is fine. Select the facts that explain the change rather than compressing every detail into dense sentences.

Adapt the content, not the format. For a bug, name the failing behavior and give the trigger and before/after result. For a refactor, name the concrete difficulty and what was checked to preserve behavior. For a feature, say what someone can now do. For performance, use measured results and conditions when available. Other changes use the same problem, change, evidence order.

## Voice

Write as a colleague asking for a quick look. Use concrete subjects and ordinary verbs: "A blocked read outlasts the deadline" and "The fix times out the read." Avoid "This PR introduces," "ensures," "streamlines," "improves maintainability," and generic claims of safety or quality. Explain the actual effect instead. Do not add enthusiasm, fake urgency, em dashes, review-report language, or commentary about your writing process.

Leave out file lists, internal identifiers, test-command inventories, and rollout mechanics unless a particular detail changes how the reviewer should assess the work. Include a limitation when omitting it would misrepresent the change. Do not add a generic request to check correctness or edge cases.

Before returning, cut repeated facts and sentences that could describe almost any change. Each bullet should make one clear point, understandable on the first read. Return only the message, without a code fence.

## Examples

Use these as style examples, not facts to reuse.

Could someone please review [#4328](https://example.com/pull/4328)?

- A stalled output read can keep a prompt waiting past its deadline.
- The fix enforces the deadline while the read is blocked. The command may continue running after output following stops.
- Pausing Valkey for 3 seconds with a 250 ms deadline previously returned a late completion; it now returns a timeout. Verified with real Valkey.

Could someone please review [#1234](https://example.com/pull/1234)?

- Three callers handle deadlines separately, so each fix needs the same change in three places.
- They now share one deadline handler, with no intended behavior change.
- The existing timeout and cancellation tests pass for all three callers.
