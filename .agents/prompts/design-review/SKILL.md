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

Do not give architecture vibes. Show the symptom, the code evidence, and the smallest useful design move.

Preserve domain meaning. Do not collapse real concepts or hide real invariants just to make code look simpler.

Read enough callers, callees, data flow, ownership, domain terms, and conventions before judging.

Look for symptoms:
- change amplification
- cognitive load
- unknown unknowns

Map symptoms to design problems only when evidenced:
- shallow module
- information leakage
- pass-through method
- conjoined methods
- temporal decomposition
- overexposed API
- generic interface

Compare 2-3 options, including no change when credible. Prefer deep interfaces that hide real complexity without erasing domain language. Recommend restraint when abstraction churn costs more than it saves.

Output:
```text
Scope: <scope>
Read: <files/artifacts>
Symptoms:
- <symptom> — <evidence>
Options:
- No change: <trade-off>
- <option>: <trade-off>
Recommendation:
- <smallest useful move>
Domain check:
- <preserved distinction/invariant>
Verdict: <keep | adjust | refactor | defer>
```

If the design is good, say why briefly.
