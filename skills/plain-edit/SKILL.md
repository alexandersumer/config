---
name: plain-edit
description: Tighten existing writing while preserving meaning, structure, and voice. Use for plain-language cleanup or removing AI-style phrasing. Use write-up for drafting or restructuring, and one-clear-sentence or one-clear-paragraph for compression.
---

Edit the target text so it reads as if a clear human wrote it. Return the edited text, not a critique of it.

This is mechanical style cleanup: remove the AI-default voice and tighten the prose. Do not rewrite the argument, restructure the document, or fact-check it.

## Source selection

1. Explicit source text or `$ARGUMENTS` in the current request.
2. With no new text, the most recent substantive draft, answer, pasted block, or document.
3. If there is genuinely no text to edit, ask in one short sentence for it.

## Preserve

- Meaning, facts, claims, numbers, names, and citations. Do not add, drop, or soften substance.
- Structure and order. Do not reorganize sections, merge paragraphs, or change the argument's shape; light filler cuts within a paragraph are fine.
- Voice and register, formal or casual. Plain does not mean flat, blunt, or stripped of personality, jokes, or considered word choices.
- Format and markup exactly: headings, lists, code, links, tables, quotes. Strip only stray markup that was clearly an error.
- Domain terms of art (see "Keep what is load-bearing").

If the text needs real restructuring, fact-checking, or a fresh draft, say that is `write-up` rather than silently doing it.

## What to cut and fix

Weight clusters over singletons: one flagged word in a long passage is usually fine; three decorative ones in a sentence is slop anywhere.

**Inflated vocabulary.** Replace puffed-up words with the plain one. Common offenders, most persistent first: enhance, leverage, utilize, robust, seamless, comprehensive, showcase, underscore, foster, harness, navigate, streamline, empower, elevate, unlock, facilitate, ensure, pivotal, crucial, vital, intricate, nuanced, multifaceted, meticulous, vibrant, holistic, dynamic, transformative, innovative. Borrowed-grandeur nouns: tapestry, landscape, realm, testament, beacon, cornerstone, ecosystem, journey, blueprint, frontier, nexus. Also flag lands, boundary, and drift, subject to the load-bearing tests below. Prefer use, build, support, handle, improve, make sure, key, detailed.

**Filler openers and closers.** Cut throat-clearing and scene-setting: "It's worth noting that", "It's important to note", "When it comes to", "In today's fast-paced world", "In an era where", "At its core", "Needless to say". Cut wrap-up tags: "In conclusion", "In summary", "Overall", "At the end of the day", "The bottom line is". Start with the real first sentence; end on the last real point.

**Inflation collocations.** "plays a vital role", "a testament to", "shed light on", "pave the way", "navigate the complexities", "a wide array of", "a plethora of", "unlock your potential", "drive meaningful impact". Replace with the plain claim.

**Structural tells**, which survive paraphrase and matter most:

- The negation flip: "It's not just X, it's Y", "This isn't merely X, it's Y", "It's not about X, it's about Y". State Y directly and drop the strawman. If X carries real information, give it its own sentence.
- Rule-of-three padding: "fast, reliable, and scalable". Keep the items that carry distinct meaning; cut the synonym riding along for rhythm.
- "From X to Y" sweeps that imply a spectrum the text never earns. Name the real scope.
- Aphorism and fortune-cookie closers: "Sometimes the smallest change makes the biggest difference." Delete, or replace with the concrete point.
- "Not only... but also". Join with "and" or split into two sentences.
- Transition-word overuse: Moreover, Furthermore, Additionally, Notably opening many paragraphs. Delete most and let the order carry the link.
- Hedging stacks: "it could be argued that this may potentially help". State the claim once; keep one hedge only where the uncertainty is real.
- Participial tails that restate the clause: "...expanded overseas, marking a pivotal moment." Cut the "-ing" tail.
- Sycophancy and motivational-poster tone. Delete.

**Punctuation.** Convert by function rather than deleting blindly:

- Em dash: parenthetical aside becomes commas or parentheses; a setup-then-reveal becomes a period or a comma; two linked clauses become two sentences or join with because, since, and, or but. Keep at most one genuinely emphatic dash if the text needs it.
- Semicolon: split into two sentences, or use a comma plus conjunction.
- Colon used for a dramatic reveal: fold into one sentence or write it plainly. Keep colons that introduce a real list or definition.

## Keep what is load-bearing

Flagged words are often legitimate technical terms. Before cutting one, run two tests:

1. Substitution: swap in a plain synonym. If meaning is unchanged, it was decorative, so cut it. If the sentence becomes wrong or vaguer, it is load-bearing, so keep it.
2. Specificity: is the word naming a defined referent, or dressing up a generic claim? Defined referent stays.

Examples: keep "decision boundary", "domain boundary between services", "model drift", "config drift", "attack surface", "robust standard errors", "threat landscape" in security. Cut "pushing the boundaries", "society drifts toward", "robust solution", "ever-evolving landscape", "leverage our expertise". When unsure, keep the word.

## Do not overcorrect

- Do not chop everything into short sentences; natural prose varies sentence and paragraph length.
- Do not add grammar errors, slang, or fake casualness to seem human.
- Do not introduce new claims or examples to fill space you cut.

## Output rules

Return only the edited text, in the source's format: no preamble, no "here is the edited version", no change summary. If the user asks what changed, add a short bulleted list of edit categories after the text.

## Before replying

Revise once and check that meaning, facts, structure, and voice match the original; no listed tells remain; em dashes, semicolons, and reveal colons are converted by function; every flagged word that stayed passes the substitution test; and the result is tighter without losing substance.
