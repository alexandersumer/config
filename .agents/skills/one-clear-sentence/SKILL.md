---
name: one-clear-sentence
description: Rewrite a source or relevant takeaway as exactly one useful, self-contained, plain sentence. Use when the user explicitly asks for one sentence, a single-sentence rewrite, or compression to one concise takeaway. Use plain-edit for multi-sentence simplification, de-academicizing, or punctuation cleanup.
---

Write exactly one self-contained sentence and nothing else.

Your primary job is to compress the user's current target into the single most useful sentence, not the shortest possible sentence and not an acknowledgement of the style request. Optimize for immediate clarity over brevity when those conflict.

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

## Clarity floor

The sentence must stand on its own for a reader who has not seen the source. If the source includes them, keep the concrete subject, the specific problem or decision, the causal mechanism or blocker, and the consequence or next action.

For technical, operational, planning, or incident text, do not compress away the mechanism that explains why the issue happens or why the action matters. A good sentence often needs the system, trigger, failure mode, timing or boundary condition, and practical response.

Avoid vague summaries that force obvious follow-up questions like which system, what failed, why it failed, when it happens, what to do next, or what condition changes the answer. If a reader would need to ask one of those questions and the source answers it, revise the sentence to include that answer.

Do not default to tiny 8 to 15 word summaries for dense source material. There is no target word count, but dense technical or planning sources often need 25 to 45 words to be useful.

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

Prefer the shortest phrasing that still carries the useful substance, but never remove the concrete detail needed for the sentence to be understood and acted on without a follow-up.
