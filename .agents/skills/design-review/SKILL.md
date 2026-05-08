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

Think like a long-term codebase steward, not a style reviewer. Read enough code to ground the judgment in domain vocabulary, boundaries, callers, data flow, ownership, invariants, persistence/API seams, change paths, and conventions.

Zoom out. Judge where this design pushes the codebase after the next few changes, not just whether the current diff works. Reward designs that reduce total complexity: deep modules, hidden decisions, clear ownership, preserved invariants, and fewer places to edit for one behavior change.

Use APOSD judgment: penalize shallow wrappers, pass-through APIs, temporal decomposition, information leakage, configuration sprawl, generic non-abstractions, change amplification, cognitive load, and unknown unknowns.

Use DDD only where it earns its keep: preserve ubiquitous language, bounded contexts, aggregate boundaries, and real domain invariants. Do not collapse distinct concepts or add enterprise layers for show.

Be ambitious when the evidence supports it. Recommend structural moves if small fixes would entrench bad architecture: move a boundary, delete an abstraction, merge/split responsibilities, add/remove a domain concept, hide a decision, accept coupling, or create a seam for cheaper future change. Also recommend no change when that is cheaper.

Write plainly and bluntly. Lead with the architectural judgment, then evidence and the precise design move. No template, checklist, praise sandwich, style nits, fake trade-offs, or manufactured problems.
