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

Output to this chat window only. Never write to disk.

Audience was in the discussion. Lead with the conclusion, then the reasoning, then what is open. Prose by default. Headings, bullets, and tables only when the content is genuinely structured. As short as the content allows.

Verify every load-bearing fact against source material directly: code, official docs, primary specs, the referenced files. Do not rely on chat summaries, team discussions, or your own earlier assertions. Omit any fact you cannot verify. Do not guess, estimate, or invent timelines or phases.

Quote sources verbatim when citing a specific fact. Identify the source inline.

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
