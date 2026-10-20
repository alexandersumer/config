---
name: request-review
description: Draft a concise message asking someone to review a pull request, adapted to the change and grounded in available evidence. Use for review requests, not performing a review or writing the pull request description.
---

Write a ready-to-copy review request in plain, natural language. Draft only; do not post or send it.

## Ground the message

Use the supplied pull request, diff, and relevant conversation evidence. If needed, inspect the target pull request or local changes with read-only tools. Describe the whole change being reviewed, not just the latest edit. Do not include unrelated local work in a request for an existing pull request.

Identify the main reason for the change, its observable effect, and the evidence available. Adapt to that purpose rather than mechanically classifying files or assigning a template to each change type. For mixed changes, lead with the main purpose and mention secondary changes only when they affect review.

Do not invent motivation, reproduction steps, measurements, test results, rollout status, or preserved behavior. Distinguish observed results from expectations. A test in the diff is coverage, not evidence that it passed. Use verification evidence only when it applies to the version and behavior being described. If an important check is still pending, say so briefly; omit empty verification claims. If the target or essential context cannot be established, ask one focused question instead of fabricating a finished request.

## Keep a recognizable shape

Open with: `Could someone please review [#<number>](<URL>)?`

Follow with roughly three short bullets, usually in this order:

- Why the change is needed: the broken behavior, missing capability, or concrete problem with the current approach.
- The evidence or example that makes it understandable: a repro and before/after behavior for a bug; a usage example for a feature; the structural problem and behavior-preservation checks for a refactor; measured results and conditions for a performance change.
- What changes: the fix or approach, with any limitation, compatibility concern, or rollout detail that affects review.

Keep those questions consistent, but combine, reorder, or omit bullets when that reads better. Reliability, security, dependency updates, tests, and cleanup follow the same questions; do not force a repro or a benchmark where neither helps. For a refactor, explain why the current structure causes trouble rather than claiming it is simply cleaner. For a feature, explain what someone can now do. Include an explicit request to examine a particular decision only when there is a concrete reason.

## Wording

Use direct factual sentences and familiar words. Start with the behavior or problem, not "This PR," "The goal is," or a change-type label. Prefer "A blocked read can outlast the deadline" to "The bug is that a blocked read can outlast the deadline." Name implementation details only when they help the reviewer understand the change.

Keep the message short enough to read at a glance. Use nested before/after bullets when they make a repro easier to scan. Avoid headings, checklists, generic benefits, promotional language, fake urgency, em dashes, and claims such as "robust," "seamless," or "improves maintainability" without a concrete explanation. Mention flags only when their scope or state matters. Do not pad the message to reach a bullet count.

Reread it as one message: the reason, evidence, and approach should be clear without opening the diff. Remove repetition and anything that does not help someone assess the change. Return only the review request, without a preamble, explanation, or surrounding code fence.
