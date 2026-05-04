---
name: philosophy-of-software-design
description: Use when reviewing code, diffs, PRs, or designs for long-term complexity, module boundaries, interface depth, or abstraction fit. Trigger on phrases like "review this design," "is this the right abstraction," "should this be split," "is this getting too complex," or any architecture feedback request — even without the words "design review." Not for style polish or isolated bugs.
argument-hint: "[optional: design, diff, file, plan, or scope]"
inputs:
  - name: scope
    label: Design scope
    description: Optional design, diff, file, plan, or scope to evaluate. Leave empty to infer from conversation or current branch.
    type: string
    required: false
---

<intent>
Reduce future complexity by diagnosing symptoms, naming red flags, and recommending the smallest change that improves the interface-to-implementation ratio.
</intent>

<hard_constraint>
Preserve domain meaning. Veto any move that flattens meaningful business distinctions, hides real invariants, or erases language the domain depends on, even if the code shape looks simpler.
For example, do not collapse Invoice and Receipt into Document just because their fields overlap.
</hard_constraint>

<process>
1. Resolve the scope from `$ARGUMENTS`, recent conversation, a design artifact, a file path, or the current branch diff. State assumptions inline and proceed unless a wrong assumption would flip the recommendation; only then ask one focused question.
2. Read enough context to understand callers, callees, data flow, ownership boundaries, domain terms, and existing conventions before judging.
3. Measure complexity by symptoms first:
   - Change amplification: how many places must change for one conceptual change?
   - Cognitive load: how much must a reader know at once to use or modify this safely?
   - Unknown unknowns: what hidden dependencies or non-obvious constraints could surprise a maintainer?

Map symptoms to candidates — change amplification often points to information leakage or conjoined methods; cognitive load often points to shallow modules or overexposure.
</process>

<stance>
- Diagnose named red flags only after symptoms are visible. Prefer pattern names and evidence over taste claims.
- Design it twice: sketch 2–3 plausible options, including keeping the current design when credible. Prefer the deepest interface for its implementation cost.
- Apply restraint: recommend no change when complexity is local, understandable, and cheaper than abstraction churn.
- Report findings collaboratively as questions and trade-offs, not mandates. If there is little signal, keep the review short.
</stance>

<red_flags>
- Shallow module: interface complexity ≈ implementation complexity.
- Information leakage: callers must know internal representation, ordering, storage, protocol, or lifecycle details.
- Pass-through method: forwards parameters/results without adding abstraction, policy, or simplification.
- Conjoined methods: methods must be called together or in a specific order the interface does not enforce.
- Temporal decomposition: modules are split by execution order rather than stable concepts or responsibilities.
- Overexposure: public API, types, fields, options, or flags expose choices most callers should not make.
- Generic interfaces: names or shapes are so abstract that domain intent and invariants disappear.
- Comments that restate code: comments repeat mechanics instead of capturing non-obvious intent, constraints, or rationale.
</red_flags>

<severity>
Classify each finding by impact and confidence:
- Blocking: likely to create significant change amplification, cognitive load, or unknown unknowns; address before committing to the design.
- Important: meaningful design risk with credible future cost; fix soon or explicitly accept the trade-off.
- Suggestion: improvement that could reduce complexity, but the current design is serviceable.
- Nit: small naming/API/documentation clarity issue that should not distract from larger design questions.
</severity>

<principles>
- Deep modules beat shallow modules: simple interfaces should hide meaningful implementation complexity.
- Interfaces matter most: improve what callers must know, not only how internals are written.
- Pull complexity downward when it simplifies many callers; do not leak implementation details upward.
- Generalize only from repeated pressure; one use case usually calls for a concrete design, not a framework.
- Domain clarity is a veto, not a preference: real business concepts, invariants, and language stay explicit.
- Avoid tactical patches that solve the immediate symptom while making the system harder to reason about.
</principles>

<worked_example>
```text
Code shape:
- Callers construct CacheKey(user.id, org.id, locale, featureFlags.hash)
- Callers then call cache.get(key), deserializeUser(), and check freshness.

Symptom observed:
- Cognitive load: every caller must know key structure, serialization, and freshness rules.

Red flag(s):
- Shallow module: Cache exposes almost as much policy as it hides.
- Information leakage: callers know the key representation and freshness protocol.

Options:
- Keep as-is if only one caller exists and policy is still volatile.
- Add UserCache.getFreshUser(user, org, locale) to own keying, deserialization, and freshness.
- Split freshness into a separate policy only if multiple caches already share it.

Recommendation:
- Would moving key construction and freshness behind UserCache reduce caller knowledge without hiding domain rules?
```
</worked_example>

<tone>
Write like a collaborator reviewing design with the author. Prefer questions such as “Would this reduce caller knowledge?”, “What invariant owns this rule?”, or “Could this sit behind the interface?” Use mandates only for blocking issues with strong evidence.
</tone>

<output_format>
For substantive reviews, use this structure. Compress or omit empty sections when the answer is obvious.

```text
Scope: <scope>
Assumption: <inline assumption, if any>
Design read: <key files/artifacts>

Symptom observed:
- <symptom> — <evidence with code reference>

Red flag(s):
- [<severity>] <red flag name> — <evidence with code reference>

Options:
- No change: <when the current design is acceptable + trade-off>
- Option A: <alternative + trade-off>
- Option B: <alternative + trade-off>

Recommendation:
- <question or trade-off framing the smallest useful design move>

Domain preservation check:
- <why meaningful distinctions are preserved, or why a tempting simplification is vetoed>

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
