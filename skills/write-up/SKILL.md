---
name: write-up
description: Produce a tight, fact-checked write-up adapted to the user's purpose, audience, and argument. Use for recaps, decision summaries, investigation notes, recommendations, status updates, and synthesis.
---

Write the requested write-up in chat only. Apply `focus` or `$ARGUMENTS` if provided.

## Aim

Produce clear writing that helps the reader understand the point, the evidence, the trade-off, and the next implication. Do not default to a fixed template. Do not open with a heading named `Conclusion`.

## Grounding

- Verify load-bearing facts against primary sources when available: code, diffs, official docs, specs, referenced files, tickets, logs, or prior messages.
- Omit or qualify claims you cannot verify. Do not turn uncertainty into confident prose.
- Preserve the user's actual argument and intent. If the source material has competing arguments, present the tension rather than flattening it into a neutral recap.

## Structure

Choose the structure from the job, not habit:

- **Decision or recommendation:** lead with the recommended action and why it matters, then the trade-offs and next step.
- **Investigation or incident:** lead with what happened and current state, then cause, impact, evidence, and remaining risk.
- **Status update:** lead with progress and whether anything needs attention, then blockers, risks, and next actions.
- **Design or strategy synthesis:** lead with the central thesis, then supporting reasoning, constraints, and implications.
- **Meeting or discussion recap:** lead with the decisions and open questions, not chronology.

Use headings only when they improve scanning. Prefer natural, specific headings over generic labels. Never force sections named conclusion, reasoning, or open items unless they are clearly the best fit.

## Style

- Start with the strongest useful sentence, not a throat-clearing opener.
- Prefer a few focused paragraphs over a long bullet inventory.
- Keep technical details only when they change the decision, explain the cause, support a claim, or tell someone what to do next.
- Summarize implementation minutiae at the level of consequence: behavior, risk, ownership, rollout, compatibility, or user impact.
- Use bullets only for decisions, actions, risks, options, or short evidence lists where scanning beats prose.
- Make every paragraph carry one claim plus the minimum support needed.
- Cut repetition, generic setup, obvious context, and decorative transitions.
- Be direct and active. Use plain words.

## Editing pass before replying

Before sending, revise once:

1. Remove any default-template feel, especially an opening `Conclusion` heading.
2. Delete sentences that do not change the reader's understanding or action.
3. Collapse technical lists into higher-level paragraphs unless the details are necessary.
4. Check that the first paragraph answers the user's real question.
5. Check that caveats and open questions are material, not defensive.

Forbidden: em dashes, emojis, sycophancy, rule-of-three padding, aphorisms, filler openers, unsupported estimates, restating the user's question, and AI tells like `delve`, `leverage`, `robust`, `comprehensive`, `seamless`, `underscore`, `tapestry`, `realm`, `landscape`.
