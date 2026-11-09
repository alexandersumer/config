---
name: write-review-request
description: Write a short, clear review request for an existing pull request that a teammate can understand without reading the diff, covering the behavior, reason, and any repro, validation, or rollout facts that matter. Use for review requests, not performing reviews, writing PR descriptions, or assigning reviewers.
---

Write the message only. Do not send it, create a pull request, or assign reviewers.

## Length is a hard limit

Keep the message under 80 words, excluding the PR link. Use two to four bullets, one sentence each. A reviewer reads this in a chat thread; the PR description holds the detail. If a draft runs over, cut bullets and clauses until it fits. Do not trade the limit for completeness. Only an explicit user request for more detail lifts it.

## Ground the message

Use the supplied PR, diff, and conversation. Read the target with read-only tools when needed. Describe the whole PR's purpose, not its latest edit, and exclude unrelated local work. Ask one short question only if the target cannot be resolved.

Never invent motivation, repro results, test results, flag states, or rollout guarantees. A test in the diff is not a passing test. Do not claim something is verified unless the evidence shows it.

## Format

Open with `Could someone please review [#<number>](<URL>)?`, a blank line, then bullets with bold labels:

- **What:** The resulting behavior, in plain words. Always include.
- **Why:** The problem it fixes or the need it meets. Always include.
- **Repro** or **Example:** The trigger and before/after outcome, only when you have executed evidence or a one-line trigger that makes the change obvious.
- **Validation:** What was checked and where, only when it adds evidence beyond Repro.
- **Rollout:** The flag or control that activates it and how to turn it off, only for gated, risky, or hard-to-reverse changes.
- **Review:** One specific question, only when a decision needs the reviewer's judgment.

Pick the two to four bullets that matter most. When an important fact is unknown, such as rollout control on a risky change, say so in a few words ("Rollout control: none found."). Do not list everything that was not verified.

## Style

Write as a colleague asking a favor. Use concrete subjects and ordinary verbs. Name the actual behavior ("close the connection when the client disconnects") instead of abstractions ("improve lifecycle handling"). Keep an identifier only when the reviewer needs it, such as the exact flag. Skip "This PR introduces," "ensures," "robust," safety claims, file lists, test commands, em dashes, and closing lines.

Before returning, check once: under 80 words, each bullet says one new thing, nothing claims more than the evidence shows. Return only the ready-to-copy message, with no code fence or commentary.

## Examples

Fictional; never reuse their facts or flag names.

Could someone please review [#4328](https://example.com/pull/4328)?

- **What:** Show a payment as pending when the provider times out, instead of as failed.
- **Why:** Customers see a failure after being charged and pay again.
- **Rollout:** Behind `payment-pending`, staff-only; turning it off restores the old display.

Could someone please review [#1234](https://example.com/pull/1234)?

- **What:** The three importers now share one date parser and keep their accepted formats.
- **Why:** Date fixes had to be copied three times and the copies drifted.

Could someone please review [#5678](https://example.com/pull/5678)?

- **What:** Release the export slot when a client disconnects mid-export.
- **Why:** Abandoned exports can hold every slot and block new ones.
- **Repro:** Not run yet; disconnect during an export, then start another.
