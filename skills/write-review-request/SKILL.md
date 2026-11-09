---
name: write-review-request
description: Write a short, clear review request for an existing pull request that a teammate can understand without reading the diff, covering what changes, why, and any testing, rollout, or review facts that matter. Use for review requests, not performing reviews, writing PR descriptions, or assigning reviewers.
---

Write the message only. Do not send it, create a pull request, or assign reviewers.

## Purpose

A review request is a chat message. In about ten seconds the reader should know what changes, why it matters, how risky it is, and what you want from them. The PR description holds everything else.

When rules conflict, follow this order: accuracy, then the length limit, then completeness.

## Hard limits

- Under 80 words for one PR, excluding links.
- Two to four bullets, one sentence each, about 20 words or fewer per bullet.
- No nested lists, file lists, test commands, or closing line.

Do not go past the limit to add detail. Lift it only when the user asks for more. When asked to shorten, cut bullets before trimming words.

## Ground it

Use the supplied PR, diff, and conversation. Read the PR with read-only tools when needed. Describe the whole PR, not its latest commit, and leave out unrelated local work. If you cannot find the PR, ask one short question.

Never invent motivation, results, flag states, or rollout guarantees. A test in the diff is not a passing test, and suggested steps are not a run repro. Say "passed in CI" or "passed locally" only when you have seen it.

## Format

Start with `Could someone please review [#<number>](<URL>)?`, then a blank line, then bullets from this list in this order:

- **What:** The resulting behavior, described as what now happens. Always include.
- **Why:** The trigger and the harm or need it fixes. Always include.
- **Tested:** The strongest evidence you have and where it ran. Include only when it is real evidence beyond "tests added."
- **Rollout:** What turns the change on and how to turn it off. Include for gated, risky, or hard-to-reverse changes. If a risky change has no control, say "No flag; revert to roll back."
- **Review:** One specific question when a decision needs the reviewer's judgment. Name the decision, such as "Is 30s the right timeout for slow uploads?"

Docs, refactors, and small fixes usually need only What and Why.

For several PRs, write one ask line, then one bullet per PR: `[#<number>](<URL>): <what it does and why>`. Keep each bullet under 25 words.

## Write it plainly

- Write for a teammate who has not seen the work. Explain internal names by what they do, or drop them.
- Describe behavior ("close the connection when the client disconnects"), not abstractions ("improve lifecycle handling").
- Keep an identifier only when the reviewer needs it, such as the exact flag name.
- Avoid "This PR," "ensures," "robust," "streamlines," safety claims, hedging, and em dashes.

Before returning, check that the message is under the limit, that every bullet says something new, and that no claim goes beyond the evidence. Return only the ready-to-copy message, with no code fence or commentary.

## Examples

These are fictional. Never reuse their facts or flag names.

Could someone please review [#4328](https://example.com/pull/4328)?

- **What:** Show a payment as pending when the provider times out, instead of failed.
- **Why:** Customers see a failure after being charged, and some pay twice.
- **Rollout:** Behind `payment-pending`, staff-only; turning it off restores the old screen.
- **Review:** Is "pending" clear enough while we wait for the provider?

Could someone please review [#1234](https://example.com/pull/1234)?

- **What:** The three importers now share one date parser and keep their accepted formats.
- **Why:** Date fixes had to be copied three times, and the copies drifted.

Could someone please review these?

- [#5678](https://example.com/pull/5678): Free the export slot when a client disconnects, so abandoned exports stop blocking new ones.
- [#5679](https://example.com/pull/5679): Alert when export slots stay full for 10 minutes.
