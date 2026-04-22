---
name: write-up
description: Document the preceding discussion as clear, fact-checked prose in chat
argument-hint: "[optional: focus or angle]"
inputs:
  - name: focus
    label: Focus or angle
    description: Optional aspect of the discussion to emphasize. Leave empty to cover the whole discussion.
    type: string
    required: false
---

Write a document of the preceding discussion. Apply the focus below if provided.

$ARGUMENTS

Output to this chat window only. Do not write to disk.

Audience was in the discussion. Structure: conclusion first, then reasoning, then what is open. Prose by default. Use headings, bullets, or tables only when the content is genuinely structured. Length matches content; cut any word that does not earn its place.

Verify every load-bearing fact against primary source material: the code, official docs, primary specs, the referenced files. Treat chat summaries, team discussions, and your own earlier assertions as unverified. Omit any fact you cannot verify against a primary source.

When citing a specific fact, quote the source verbatim and identify it inline.

Forbidden:
- em dashes
- emojis
- hedging ("it seems", "potentially", "might be worth", "arguably")
- AI tells ("delve", "leverage", "robust", "comprehensive", "seamless", "underscore", "tapestry", "realm", "landscape")
- filler openers ("Overall", "In summary", "It's worth noting", "Furthermore", "Moreover")
- sycophancy ("Great question", "Certainly", "Of course")
- rule-of-three padding
- aphorisms
- estimation and phasing ("roughly", "approximately", "phase 1", "next steps") unless the source says so
- verbose specifics with no information value (absolute file paths when a name suffices, file counts, line counts)
- restating the user's question

Plain, direct, active. Cut every word that does not earn its place.
