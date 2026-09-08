---
name: one-clear-paragraph
description: Rewrite a source or relevant takeaway as exactly one useful, self-contained paragraph in plain, natural prose. Use when the user asks for one clear paragraph, a single-paragraph rewrite, or a paragraph-length synthesis. Use one-clear-sentence for exactly one sentence and plain-edit for style cleanup that preserves the source structure.
---

Write exactly one self-contained paragraph and nothing else.

Compress the user's current target into a useful, connected explanation. Prioritize immediate clarity and the substance the reader needs over a fixed word or sentence count.

## Source selection

Use the most specific available source in this order:

1. Explicit source text or `$ARGUMENTS` from the current request.
2. The most recent substantive assistant answer, tool result, pasted block, draft, or discussion when invoked with no new source text.
3. The current user request itself when there is no separate source to rewrite.

If there is no source or topic to rewrite, ask one short sentence for the text to condense.

## Select and connect

Preserve the user's meaning, facts, intended audience, and important uncertainty. Select the central outcome, decision, explanation, blocker, or next action, then retain the supporting detail needed to understand it. Do not invent facts or examples.

Lead with the main point. Let each following sentence explain its reason, mechanism, consequence, or practical response. Reorder and combine source material when that makes the synthesis clearer. Drop incidental inventories, repeated evidence, and secondary examples.

Make the paragraph stand on its own for someone who has not seen the source. Name the concrete subject and retain conditions, qualifications, and causal details that change the conclusion. For technical or operational material, keep the relevant trigger, failure mechanism, and response. Preserve citations attached to retained claims when supplied.

Use several connected sentences when the substance needs them. Avoid both a list disguised as prose and a single overloaded sentence. Stop when the point is clear without adding a closing sentence that repeats it.

## Plain style

Use concrete subjects, plain verbs, active voice, and natural phrasing. Match the author's register and preserve their personality without adding slang or fake casualness.

Cut filler openers and closers, inflated vocabulary, praise, slogans, and decorative endings. Replace phrases such as "plays a vital role" with the actual claim. State the point directly instead of using "not X, but Y" framing unless the distinction carries essential meaning.

Remove synonym lists added for rhythm, unsupported "from X to Y" sweeps, and clauses that merely repeat a claim. Keep real uncertainty while cutting stacked hedges. Connect sentences through their substance and order rather than repeated "Moreover" or "Furthermore" transitions.

Keep precise domain terms such as "model drift" or "decision boundary" when a plainer substitute would change the meaning or make it vague. Judge words by their function rather than a blacklist.

Prefer periods, commas, and conjunctions to em dashes, semicolons, and dramatic colons. Preserve punctuation required by literal code, names, or citations. Vary sentence length so the paragraph reads naturally rather than becoming choppy.

## Output and final check

Return only the paragraph, with no heading, label, bullet points, preamble, alternatives, or explanation of the edits unless the user explicitly requests a wrapper. Do not acknowledge the style request or promise future behavior.

Revise once to check that there is exactly one paragraph, the main point is clear immediately, each sentence adds useful substance, and the result preserves the source's essential meaning and uncertainty. Remove filler without removing details the reader needs to understand or act on the point.
