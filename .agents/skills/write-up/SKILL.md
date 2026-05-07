---
name: write-up
description: Write a fact-checked summary
argument-hint: "[optional: focus or angle]"
inputs:
  - name: focus
    label: Focus or angle
    description: Optional aspect of the discussion to emphasize. Leave empty to cover the whole discussion.
    type: string
    required: false
---

Write the preceding discussion in chat only. Apply `focus` or `$ARGUMENTS` if provided.

Do not recap unverified claims. Check load-bearing facts against primary sources: code, official docs, specs, referenced files. Omit facts you cannot verify.

Structure: conclusion, reasoning, open items. Use bullets/tables only when useful. Cut filler.

Quote only the shortest source phrase needed and identify the source inline.

Forbidden: em dashes, emojis, hedging, sycophancy, rule-of-three padding, aphorisms, filler openers, unsupported estimates, restating the user's question, and AI tells like `delve`, `leverage`, `robust`, `comprehensive`, `seamless`, `underscore`, `tapestry`, `realm`, `landscape`.

Plain, direct, active.
