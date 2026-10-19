---
name: comment-on-pull-request
description: Post focused pull request feedback in plain, natural language when asked to leave or post review comments.
---

Write useful feedback a teammate can understand on the first read.

A request to post authorizes posting on the target PR. Honor draft-only requests.
Reviewing code or editing this skill does not authorize posting.

## Select the concerns

Resolve the PR from the request or current branch. Read the current diff,
description, and discussion. Validate existing findings against the current head;
inspect surrounding code or requirements as needed. Use the available direct
review workflow if a review is still needed.

Keep demonstrated defects, broken requirements, and consequential security,
reliability, or design concerns. Test requests need a specific regression missing
from current coverage. Combine findings with the same root cause. Drop speculation,
minor preferences, and concerns already fixed or adequately raised. Post nothing
when no issue warrants it.

## Write the comments

Default to one short paragraph per concern, usually two or three sentences.
Explain what can go wrong and why it matters, then the change or decision needed.
Keep investigation notes and supporting detail out of the comment unless the
author needs them to reproduce or understand the problem.

Use ordinary descriptions of behavior. The inline location already identifies
the code: filenames, line numbers, call chains, and lists of symbols usually add
nothing to the body. Include an identifier only when it distinguishes something
the author could otherwise misunderstand. Prefer the practical effect over the
internal mechanism. Keep the condition that causes the problem precise.

Too dense:

> `applySearchResponse()` updates `resultsStore.items` without validating `requestId` against `activeRequestId`, allowing out-of-order resolution to clobber state.

Prefer:

> If an older search finishes last, it replaces the results for the user's current query. Could we ignore responses from earlier searches?

Use plain American English. State confirmed behavior directly; ask a question
when a decision or missing fact calls for one. Match supplied examples of the
user's writing. Avoid headings, severity labels, stock praise, jargon, em dashes,
and forced casualness. Never invent experience or test results. Vary the phrasing
across comments; the example is not a template.

Before posting, read each comment as the author. Remove code references and
explanations that do not help them act. Rewrite any sentence that requires
unpacking a chain of identifiers. Keep the actual failure and useful detail clear.

## Post and verify

Use the provider's supported tools and the user's verified account. For company
or Bitbucket context, use the available `twg` guidance. Read live help when write
syntax is uncertain.

Refresh the head and existing threads before posting. Revalidate affected findings
if code changed. Prefer verified inline locations using the provider's commit,
path, line, and side semantics; use a general comment when no inline location fits.
Post only the selected comments. Other PR actions need their own authorization.

Read back the text, author, and location. Check an uncertain write before retrying
to avoid duplicates. Finish with links to verified comments and any exact blocker.
For drafts, return the comments with their intended locations. Report partial
success accurately.
