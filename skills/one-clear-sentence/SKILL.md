---
name: one-clear-sentence
description: Rewrite a source or relevant takeaway as exactly one useful, self-contained, plain sentence. Use when the user explicitly asks for one sentence, a single-sentence rewrite, or compression to one concise takeaway. Use one-clear-paragraph for a single-paragraph synthesis and plain-edit for style cleanup that preserves the source structure.
---

Write exactly one self-contained sentence and nothing else.

Your job is to compress the user's current target into the single most useful sentence, which is neither the shortest possible sentence nor an acknowledgement of the style request. When clarity and brevity conflict, choose clarity.

## Source selection

Use the most specific available source in this order:

1. Explicit source text or `$ARGUMENTS` from the current request.
2. The most recent substantive assistant answer, tool result, pasted block, draft, or discussion when the user invokes this skill with no new source text.
3. The current user request itself when there is no separate source to rewrite.

If there is truly no source or topic to rewrite, ask exactly one short sentence for the text to condense.

## Select

Preserve the user's meaning, facts, and intended audience, but extract the relevant takeaway rather than preserving every detail. Prefer the outcome, decision, blocker, risk, next action, or definition of done the user likely cares about. For a long list, status report, investigation, or plan, drop file inventories, examples, subtask lists, and incidental evidence unless one of them is the central point.

The sentence must stand on its own for a reader who has not seen the source. Keep the concrete subject, the problem or decision, and the consequence or next action. For technical, operational, planning, or incident text, also keep the mechanism: the system, trigger, failure mode, boundary condition, and practical response. If a reader would have to ask which system, what failed, why, when, or what to do next, and the source answers it, include the answer. Dense sources often need 25 to 45 words; there is no target count.

Do not invent detail, hide important uncertainty, or soften nuance to save words.

## Style

Write a normal full sentence in a natural tone, not a headline or slogan. Use concrete subjects, plain verbs, and active voice, and match the author's register without fake casualness. Join essential clauses with conjunctions rather than cramming incidental detail into a run-on.

Cut filler openers, inflated vocabulary, slogans, praise, decorative endings, synonym lists added for rhythm, unsupported "from X to Y" sweeps, and stacked hedges. Replace phrases such as "leverage our expertise" with the actual action. State the point directly instead of using "not X, but Y" framing unless the distinction carries essential meaning. Keep precise domain terms such as "model drift" or "decision boundary" when a plainer word would change the meaning.

Use no em dashes, colons, or semicolons.

## Output

Do not acknowledge, confirm, promise future behavior, or say anything like "Got it", "Sure", or "I will use that style". Do not add headings, labels, quotes, bullet points, explanations, alternatives, or preamble unless the user explicitly asks for that exact wrapper.

Before replying, check that the output is exactly one sentence that captures what the user most cares about, reads immediately, and is specific enough to act on without a follow-up question.
