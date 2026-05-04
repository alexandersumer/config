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

Review the design for future complexity: module boundaries, interface depth, information hiding, and abstraction fit.

Do not satisfy this with architecture-flavored taste. The known failure mode is naming red flags without proving user pain, or flattening real domain concepts because the code would look simpler. Diagnose symptoms first. Preserve domain meaning.

Hard veto: do not collapse meaningful business distinctions, hide real invariants, or erase language the domain depends on. Shared fields do not make Invoice and Receipt the same concept.

Process:
1. Resolve scope from `scope`, `$ARGUMENTS`, conversation, artifact, file path, or current branch diff. State assumptions inline; ask only if a wrong assumption would flip the recommendation.
2. Read enough callers, callees, data flow, ownership boundaries, domain terms, and conventions before judging.
3. Measure symptoms:
   - Change amplification: how many places change for one concept?
   - Cognitive load: what must a reader know at once?
   - Unknown unknowns: what hidden dependency, lifecycle, order, or invariant can surprise a maintainer?
4. Only then name red flags.

Red flags:
- Shallow module: interface complexity roughly equals implementation complexity.
- Information leakage: callers know representation, ordering, storage, protocol, lifecycle, or policy details.
- Pass-through method: forwards without abstraction, policy, or simplification.
- Conjoined methods: calls must happen together or in hidden order.
- Temporal decomposition: modules split by execution order instead of stable responsibility.
- Overexposure: public API exposes choices most callers should not make.
- Generic interface: abstraction erases domain intent or invariants.
- Restating comments: comments repeat mechanics instead of intent or constraints.

Severity:
- Blocking: likely significant future change amplification, cognitive load, or unknown unknowns.
- Important: real design risk with credible future cost.
- Suggestion: useful improvement; current design remains serviceable.
- Nit: small clarity issue that should not distract from design decisions.

Stance:
- Design it twice: compare 2-3 plausible options, including no change when credible.
- Prefer the deepest interface for the implementation cost.
- Pull complexity downward only when it simplifies many callers.
- Generalize from repeated pressure, not one use case.
- Recommend restraint when complexity is local and cheaper than abstraction churn.
- Phrase recommendations as trade-offs and questions unless the issue is blocking.

Output for substantive reviews:
```text
Scope: <scope>
Assumption: <if any>
Design read: <key files/artifacts>

Symptom observed:
- <symptom> — <evidence>

Red flag(s):
- [<severity>] <red flag> — <evidence>

Options:
- No change: <when acceptable + trade-off>
- Option A: <alternative + trade-off>
- Option B: <alternative + trade-off>

Recommendation:
- <smallest useful design move framed as question/trade-off>

Domain preservation check:
- <why domain distinctions/invariants remain explicit, or why a tempting simplification is vetoed>

Verdict: <keep as-is | adjust before implementation | refactor now | split/defer>
```

If the design is already good, say so briefly and name the properties that make it good. Do not write an essay when two lines would be more useful.
