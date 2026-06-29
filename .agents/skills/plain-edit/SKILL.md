---
name: plain-edit
description: Edit existing writing into plain, natural, tight prose. Strip AI-slop vocabulary, "not X, it's Y" constructions, aphorisms, filler, and excess em dashes, colons, and semicolons, while keeping the author's meaning, structure, and voice. Use when the user wants text de-slopped, humanized, made plainer, less academic or fluffy, tightened, or freed of AI tells. Use write-up to draft or restructure and fact-check new writing. Use one-clear-sentence to compress a takeaway to a single sentence.
---

Edit the target text so it reads as if a clear human wrote it. Return the edited text, not a critique of it.

Your job is mechanical style cleanup: remove the AI-default voice and tighten the prose. You are not rewriting the argument, restructuring the document, or fact-checking it. Preserve what the author meant and how they ordered it.

## Source selection

Use the most specific available source in this order:

1. Explicit source text or `$ARGUMENTS` in the current request.
2. The most recent substantive draft, answer, pasted block, or document when the user invokes this skill with no new text.
3. If there is genuinely no text to edit, ask one short sentence for the text to clean up.

## Preserve

- Meaning, facts, claims, numbers, names, and citations. Do not add, drop, or soften substance.
- The author's structure and order. Do not reorganize sections, merge paragraphs, or change the argument's shape. Light cuts of filler within a paragraph are fine.
- The author's voice and register. Match how formal or casual the original is. Plain does not mean flat, blunt, or stripped of personality.
- Format and markup exactly: headings, lists, code, links, tables, quotes. Strip only leaked or stray markup that was clearly an error.
- Domain terms of art. See "Keep what is load-bearing" before cutting any flagged word.

If the text needs real restructuring, fact-checking, or a fresh draft, that is `write-up`, not this skill. Say so rather than silently doing it.

## What to cut and fix

Weight clusters over singletons. One flagged word in a long passage is usually fine. Three decorative ones in a sentence is slop anywhere. Favor the persistent tells over dated ones.

**Inflated vocabulary.** Replace puffed-up words with the plain one. Common offenders, most still common in current models first: enhance, leverage, utilize, robust, seamless, comprehensive, showcase, underscore, foster, harness, navigate, streamline, empower, elevate, unlock, facilitate, ensure, pivotal, crucial, vital, intricate, nuanced, multifaceted, meticulous, vibrant, holistic, dynamic, transformative, innovative. Borrowed-grandeur nouns: tapestry, landscape, realm, testament, beacon, cornerstone, ecosystem, journey, blueprint, frontier, nexus. The user named lands, boundary, and drift; treat those the same way. Use plain verbs and nouns: use, build, support, handle, improve, make sure, key, detailed.

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

Flagged words are often legitimate technical terms. Do not nuke a word that names a real thing in this text's domain. Before cutting any flagged word, run two tests:

1. Substitution: swap in a plain synonym. If meaning is unchanged, it was decorative, so cut it. If the sentence becomes wrong or vaguer, it is load-bearing, so keep it.
2. Specificity: is the word naming a defined referent, or dressing up a generic claim? Defined referent stays.

Examples: keep "decision boundary", "domain boundary between services", "model drift", "config drift", "attack surface", "robust standard errors", "threat landscape" in security. Cut "pushing the boundaries", "society drifts toward", "robust solution", "ever-evolving landscape", "leverage our expertise". When unsure, keep the word and move on rather than risk changing meaning.

## Do not overcorrect

- Do not chop everything into short choppy sentences. Vary sentence and paragraph length; natural prose is uneven.
- Do not add grammar errors, slang, or fake casualness to seem human.
- Do not strip the author's real personality, jokes, or considered word choices.
- Do not introduce new claims or examples to fill space you cut.

## Output rules

Return only the edited text, in the same format as the source. No preamble, no "here is the edited version", no summary of changes unless the user asks. If the user asks what changed, give a short bulleted list of the categories of edits after the text.

## Before replying

Revise once and check:

1. Meaning, facts, structure, and voice match the original.
2. No inflated vocabulary, filler openers or closers, or AI-tell phrases remain.
3. No negation flips, tricolon padding, sweeps, or aphorisms remain.
4. Em dashes, semicolons, and stray colons are gone or converted by function.
5. Every flagged word that stayed is load-bearing under the substitution test.
6. The prose reads naturally, with varied rhythm, and is tighter than the original without losing substance.
