---
name: design-review
description: Review software design
argument-hint: "[optional: design, diff, file, plan, or scope]"
inputs:
  - name: scope
    label: Design scope
    description: Optional design, diff, file, plan, or scope to evaluate. Leave empty to infer from conversation or current branch.
    type: string
    required: false
---

Review the design in `scope`, `$ARGUMENTS`, conversation, artifact, file, or branch diff.

This is not an architecture essay. Do the work before judging.

Read until you can explain the design pressure in concrete terms: callers, callees, data flow, ownership, domain terms, invariants, change paths, and local conventions. If code can answer the question, inspect code instead of guessing.

Do not give abstract architecture advice. Do not praise or attack shapes in isolation. A design problem exists only when there is evidence: change amplification, cognitive load, hidden coupling, unknown unknowns, information leakage, shallow modules, pass-through APIs, conjoined responsibilities, temporal decomposition, overexposed APIs, or generic interfaces pretending to be reusable.

Preserve domain meaning. Do not collapse real concepts, erase useful invariants, or hide important distinctions to make code look simpler. Prefer deep interfaces only when they hide real complexity without lying about the domain.

Be suspicious of abstraction churn. Compare the real choices, including doing nothing. Do not invent fake trade-offs. Recommend no change when the existing design is cheaper than the proposed cleanup.

Your recommendation must be the smallest useful design move: the exact boundary to move, responsibility to merge or split, API to hide, invariant to expose, coupling to accept, or abstraction to delete. If the evidence is insufficient, say what you inspected and what would need to be inspected next.

Write plainly and bluntly. No template. No checklist. No padded sections. Lead with the judgment, then give the evidence that makes it hard to dismiss.

If the design is good, say why and stop. Do not manufacture a problem to justify the review.
