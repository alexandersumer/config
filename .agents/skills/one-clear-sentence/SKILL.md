---
name: one-clear-sentence
description: Rewrite the relevant takeaway as one useful, clear, plain sentence in a natural tone. Use when the user asks for one sentence, crystal clear wording, a concise takeaway, simple wording, less academic tone, or no em dashes, colons, or semicolons.
---

Write exactly one sentence and nothing else.

Your primary job is to compress the user's current target into the single most useful sentence, not the shortest possible sentence and not an acknowledgement of the style request.

## Source selection

Use the most specific available source in this order:

1. Explicit source text or `$ARGUMENTS` from the current request.
2. The most recent substantive assistant answer, tool result, pasted block, draft, or discussion when the user invokes this skill with no new source text.
3. The current user request itself when there is no separate source to rewrite.

If there is truly no source or topic to rewrite, ask exactly one short sentence for the text to condense.

## Relevance filter

Preserve the user's meaning, facts, and intended audience, but extract the relevant takeaway rather than preserving every detail. Prefer the outcome, decision, blocker, risk, next action, or definition of done the user likely cares about.

A useful sentence may be a normal full sentence, not a headline or slogan. Include the key subject, action or decision, and the consequence, reason, blocker, or condition when that context is needed for the sentence to stand on its own.

When the source is a long list, status report, investigation, or implementation plan, synthesize the highest-value point and drop file inventories, examples, subtask lists, and incidental evidence unless one of them is the central point.

Do not invent detail, hide important uncertainty, or soften important nuance just to make the sentence shorter.

## Output rules

Do not acknowledge, confirm, promise future behavior, or say anything like "Got it", "Sure", or "I will use that style".

Do not add headings, labels, quotes, bullet points, explanations, alternatives, or preamble unless the user explicitly asks for that exact wrapper.

The output must be one clear, simple sentence in a natural tone, with no em dashes, colons, or semicolons.

Keep the tone plain like `plain-edit`: use concrete words, active voice, and natural phrasing; avoid inflated vocabulary, filler, aphorisms, motivational-poster lines, and academic polish.

## Before replying

Revise once to ensure:

1. The output is exactly one sentence.
2. The sentence captures the most relevant thing the user cares about.
3. The point is immediately understandable.
4. The tone sounds natural, direct, and not academic.
5. The sentence is specific enough to be useful on its own without becoming padded or vague.
6. There are no em dashes, colons, or semicolons.

Prefer the shortest phrasing that still carries the useful substance, not the shortest phrasing possible.
