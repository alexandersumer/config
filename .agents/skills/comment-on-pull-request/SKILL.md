---
name: comment-on-pull-request
description: Consolidate review feedback into the few comments that matter, write them in clear, natural language, and post directly on a pull request under the user's authenticated account. Use when asked to leave or post review comments, including after an existing review.
---

# Comment on Pull Request

Consolidate the feedback down to the most important concerns and leave comments that read like a thoughtful colleague wrote them. Keep them plain, specific, and easy for the author to act on. Preserve the substance without turning each comment into a technical report.

Using this skill to leave comments authorizes posting on the target pull request without another confirmation. Honor requests to draft or preview instead. A request to review code, discuss this skill, or edit its instructions does not authorize posting.

## Ground and consolidate

Resolve the pull request from the supplied target or conversation, otherwise discover the current branch's pull request. Ask only if the target remains ambiguous. Read the current diff, description, and existing discussion, then enough surrounding code or requirements to validate the feedback. Comment on code present in the pull request, not unpublished local changes.

Use existing review findings when they still apply. Validate them against the current head; do not blindly publish an earlier report. If a review is needed, use the available direct review workflow, such as `review-solo`. Scale investigation to the concern rather than starting an unrelated review campaign.

Keep only concerns that materially affect whether the change should merge or how it should work: demonstrated defects, security or reliability risks, broken requirements, or consequential design problems. A test request needs a specific meaningful regression that existing coverage misses. Respect the user's requested focus.

Merge comments that share a root cause, retaining the clearest explanation and useful evidence. Keep independent concerns separate. Drop speculation, minor preferences, optional cleanup, and issues already fixed or adequately raised in existing threads. Do not manufacture comments or aim for a count. If nothing warrants a comment, post nothing.

## Write naturally

The comments must sound like the user's own thoughtful feedback to a teammate. When the conversation includes examples of their review comments, use those to match their voice. Favor concrete observations and ordinary phrasing over polished, generic review language. Do not fake human habits with slang, typos, or invented experience.

Write short, self-contained comments. Make the trigger, consequence, and requested change or decision clear. Explain what the author needs to understand, without dumping the investigation or prescribing a larger implementation than necessary.

Use plain American English. Keep technical terms and exact identifiers when they make the concern precise or help locate the fix; explain unfamiliar mechanics through their practical effect. Do not simplify away the facts that make the comment actionable.

Be direct and respectful. Ask a question when there is a real decision or missing fact, and state confirmed behavior plainly. Match confidence to the evidence. Do not turn a guess into an accusation or soften a demonstrated problem with stacked hedges.

Avoid report headings, severity labels, stock praise, filler, em dashes, forced slang, and repeated sentence templates. Omit tool narration and boilerplate about being an AI. Never invent personal experience, test results, or team agreements to sound human.

For example:

> If this save fails, we still show the document as saved, so the user won't know their changes were lost. Could we update the saved state only after the write succeeds?

> An older response can arrive after the user switches documents and overwrite the new document's content. Can we check that the response still belongs to the current document before applying it?

These illustrate the voice; do not force every comment into the same shape.

Before posting, reread the comments together. Rewrite anything that sounds like a generated review report, explains basics the author already knows, or repeats the same opener or request pattern. Keep only the words needed to make the concern clear; a natural comment can be one sentence when that is enough.

## Post and verify

Use the supported tools for the pull request's provider. For company or Bitbucket context, use the available `twg` guidance. Inspect tool schemas or live CLI help when write syntax is uncertain.

Establish that the posting account is the user's account, reusing reliable identity evidence from this session when available. Do not substitute a bot or another account, spoof an author, or claim posting under the user's name without evidence. If access or identity cannot be established, report the exact blocker.

Refresh the head and existing comments before posting. If relevant code changed, revalidate the affected feedback. Prefer inline comments at verified current diff locations, using the provider's path, commit, line, and side semantics. Use a general comment when the concern has no suitable inline location.

Post only the selected comments. Do not change approval status, merge, create tasks, resolve threads, or modify existing comments unless separately requested.

Read posted comments back to verify their text, author, and location, and retain their links or IDs. If a write has an uncertain outcome, check whether it succeeded before retrying. If that remains unknown, stop instead of risking a duplicate. Report partial success accurately.

Finish briefly with links to verified comments and any remaining blocker. For draft-only requests, return the proposed comments with their intended locations. If nothing warranted posting, say so.
