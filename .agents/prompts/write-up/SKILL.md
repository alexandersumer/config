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

Write the preceding discussion as clear prose in this chat only. Apply `focus` or `$ARGUMENTS` if provided.

Do not satisfy this by producing a polished recap of unverified claims. The known failure mode is confident prose built from memory, chat summaries, or earlier assistant assertions. Every load-bearing fact must come from primary source material or be omitted.

Structure:
- conclusion first
- reasoning second
- open items last

Use headings, bullets, or tables only when the content needs structure. Length follows substance. Cut every word that does not earn its place.

Verification:
- Check load-bearing facts against primary sources: code, official docs, primary specs, referenced files.
- Treat chat summaries, team discussion, and earlier assistant claims as unverified.
- Quote only the shortest source phrase needed for a specific fact and identify the source inline.

Forbidden:
- em dashes
- emojis
- hedging: `it seems`, `potentially`, `might be worth`, `arguably`
- AI tells: `delve`, `leverage`, `robust`, `comprehensive`, `seamless`, `underscore`, `tapestry`, `realm`, `landscape`
- filler openers: `Overall`, `In summary`, `It's worth noting`, `Furthermore`, `Moreover`
- sycophancy: `Great question`, `Certainly`, `Of course`
- rule-of-three padding
- aphorisms
- unsupported estimation or phasing
- verbose specifics with no information value
- restating the user's question

Plain, direct, active.
