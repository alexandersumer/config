---
name: philosophy-of-software-design
description: Apply John Ousterhout-style software design review to architectural shape, module boundaries, interface depth, abstraction fit, and long-term complexity. This skill should be used when reviewing designs, implementation plans, diffs, refactors, architecture, decomposition, APIs, domain models, or maintainability trade-offs — especially when the user mentions complexity, deep modules, shallow modules, interfaces, information leakage, change amplification, cognitive load, unknown unknowns, or A Philosophy of Software Design. It is not for style-only review or ordinary correctness bugs unless they reveal a design problem.
argument-hint: "[optional: design, diff, file, plan, or scope]"
inputs:
  - name: scope
    label: Design scope
    description: Optional design, diff, file, plan, or scope to evaluate. Leave empty to infer from conversation or current branch.
    type: string
    required: false
---

<trigger>
Review architectural shape, module boundaries, and interface depth when the question is long-term complexity — not style polish or isolated correctness bugs.
</trigger>

<intent>
Evaluate whether a design makes future change easier to understand. Judge complexity by observable symptoms first, diagnose causes second, and recommend the smallest move that improves the interface-to-implementation ratio without erasing real domain meaning.
</intent>

<hard_constraint>
Preserve domain meaning. Any recommendation that flattens meaningful business distinctions, hides real invariants, or collapses language the domain depends on is disqualified, even if the code shape becomes simpler.
</hard_constraint>

<workflow>
1. Resolve the scope from `$ARGUMENTS`, recent conversation, a design artifact, a file path, or the current branch diff. State assumptions inline and proceed unless a wrong assumption would flip the recommendation; only then ask one focused question.
2. Read enough surrounding context to understand callers, callees, data flow, ownership boundaries, domain terms, and existing conventions before judging.
3. Start from the three measurable symptoms of complexity:
   - Change amplification: how many places must change for one conceptual change?
   - Cognitive load: how much must a reader know at once to use or modify this safely?
   - Unknown unknowns: what hidden dependencies or non-obvious constraints could surprise a maintainer?
4. Diagnose concrete red flags only after observing symptoms. Prefer named patterns over taste claims.
5. Design it twice: sketch 2–3 plausible alternatives, including keeping the current design when credible. Choose the option with the deepest interface for its implementation cost.
6. Apply restraint: recommend no change when complexity is local, understandable, and the churn/risk of refactoring exceeds the likely win.
7. Report findings collaboratively as questions and trade-offs, not mandates. Make the recommendation easy to accept, reject, or defer.
</workflow>

<red_flags>
Use these names when they fit, and cite evidence from the code/design:
- Shallow module: the interface is nearly as complex as the implementation it hides.
- Information leakage: callers must know internal representation, ordering, storage, protocols, or lifecycle details.
- Pass-through method: a method mostly forwards parameters/results without adding abstraction, policy, or simplification.
- Conjoined methods: methods must be called together or in a specific pairing that the interface does not enforce.
- Temporal decomposition: modules are split by execution order rather than stable concepts or responsibilities.
- Overexposure: public API, types, fields, options, or flags expose choices most callers should not make.
- Generic interfaces: names or shapes are so abstract that domain intent and invariants disappear.
- Comments that restate code: comments explain mechanics the code already says instead of capturing non-obvious intent, constraints, or rationale.
</red_flags>

<severity>
Classify each finding by impact and confidence:
- Blocking: likely to create significant change amplification, cognitive load, or unknown unknowns; address before merging/committing to the design.
- Important: meaningful design risk with credible future cost; fix soon or explicitly accept the trade-off.
- Suggestion: improvement that could reduce complexity, but the current design is still serviceable.
- Nit: small naming/API/documentation clarity issue that should not distract from larger design questions.
</severity>

<principles>
- Complexity is the enemy: optimize for understandable future changes, not just today’s task.
- Deep modules are better than shallow modules: a good interface is simple while hiding meaningful internal complexity.
- Interfaces matter more than implementations: improve what callers must know, not only how internals are written.
- Pull complexity downward when it simplifies many callers; do not leak implementation details upward.
- Generalize only from repeated pressure. One use case is usually evidence for a concrete design, not a framework.
- Preserve domain clarity as a veto, not a preference: real business concepts, invariants, and language should remain explicit.
- Avoid tactical coding: patches, flags, wrappers, or conditionals that solve the immediate symptom can still make the system harder to reason about.
- Prefer no change when the design cost is local, obvious, and cheaper than the churn of abstraction.
</principles>

<tone>
Write like a collaborator reviewing design with the author. Prefer: “Would this reduce caller knowledge if…?”, “What invariant owns this rule?”, or “Could this be pulled behind the interface?” Avoid decree-like phrasing unless the severity is blocking and evidence is strong.
</tone>

<output_format>
Use this concise structure:

```text
Scope: <scope>
Assumption: <inline assumption, if any>
Design read: <key files/artifacts>

Complexity symptoms:
- Change amplification: <observed/not observed + evidence>
- Cognitive load: <observed/not observed + evidence>
- Unknown unknowns: <observed/not observed + evidence>

Findings:
- [<severity>] <red flag name> — <collaborative question or concern> — evidence: <file/ref>

Design it twice:
- Option A: <alternative + trade-off>
- Option B: <alternative + trade-off>
- Option C: <optional alternative/current design + trade-off>

Recommended design move:
- <smallest change, or “no change” if restraint applies>

Trade-offs:
- <what gets simpler>
- <what cost or risk remains>
- Domain preservation check: <why meaningful distinctions are preserved, or why a tempting move is vetoed>

Verdict: <keep as-is | adjust before implementation | refactor now | split/defer>
```

If the design is already good, say so and name the properties that make it good. If two lines are enough, write two lines.
</output_format>

<gotchas>
Avoid these common failures:
- Forcing depth onto shallow domains such as straightforward CRUD, glue code, one-off ETL, or thin integration layers.
- Generalizing from one use case before repeated pressure proves the abstraction.
- Mistaking domain richness for accidental complexity; some complexity belongs in the model because the business is complex.
- Producing an essay when the useful review is a short verdict plus one concrete design move.
</gotchas>
