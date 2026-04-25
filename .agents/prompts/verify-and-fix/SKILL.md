---
name: verify-and-fix
description: Verify scoped behavior with code review and targeted checks, then fix real defects
argument-hint: "[optional: scope, file, function, behavior, or planning artifact]"
inputs:
  - name: scope
    label: Verification scope
    description: Path, function, behavior, or planning artifact to verify. Leave empty to infer from conversation, recent changes, or relevant artifact.
    type: string
    required: false
---

Verify the important scoped behavior, then fix real defects. Be evidence-driven without turning the task into an exhaustive proof exercise.

## Resolve scope
First source that yields one: `scope` input → recent changes (`git status`, `git diff base...HEAD`) → relevant planning artifact → conversation. Emit `Scope:` and `Source:` before any verification. If ambiguous or trivially small, ask.

## Specification under test
List the important contract items with primary sources. Items without a source are open questions, not assumptions. Keep the list scoped to behavior that can realistically affect correctness or compatibility.
- Functional contract: inputs, outputs, side effects, ordering, errors.
- Invariants: pre/postconditions, state transitions, isolation.
- Boundaries: empty/null/zero/one/many, min/max, unicode, time zones, concurrency.
- Failure modes: partial failure, retry, idempotency, cancellation.
- Cross-component: API/wire formats, schema versions, compatibility.

## Empirical verification
For each important contract item, run real commands and capture enough output to support the conclusion:
- Use the repo's existing build/test/typecheck/lint commands.
- Add targeted tests for missing boundary/failure cases before claiming the item passes.
- Exercise integration paths through real entry points when they exist.
- Record `Empirical: <command> -> <result>` and `Evidence: <test or log ref>`.

## Formal verification
For each important contract item, reason from primary sources:
- Trace control, data, and error flow end to end.
- Case analysis on inputs/state space (empty, singleton, large, malformed, concurrent, adversarial).
- Cross-check types/schemas/protocols against declarations and consumers.
- Cross-check against the planning artifact and docs.
- Record `Formal: <claim> — <reasoning> — <code refs>`.

## Forbidden (these are defects in the verification itself)
- Hedging: "looks fine", "should work", "appears to handle", "likely correct".
- Asserting correctness from reading without running an exercising test.
- Claiming a test passes/covers a case without naming the test and the case.
- Mocking the system under test, then claiming the system under test works.
- Suppressions, baselines, snapshot rewrites, dependency bumps, or build/test-config edits to clear failures.
- Loosening tests, deleting cases, weakening assertions, or moving cases out of scope.
- Rewriting code so the failing test no longer applies, instead of fixing the defect.
- Concluding "no issues" without empirical evidence and code reasoning for the important scoped behavior.

## Findings
One entry per defect, ordered by severity (`critical|high|medium|low`). Spec/code disagreements and missing tests for in-scope behavior are findings, not commentary. Group symptoms with a shared root cause under one finding.

## Fix loop
For each finding, highest severity first:
1. Smallest correct fix at the root cause. No band-aids, no defensive `try/except`, no silent fallbacks.
2. Match surrounding code and test style.
3. Add or strengthen a test that fails without the fix and passes with it.
4. Re-run empirical verification for the affected items plus regression for the rest of scope.
5. Re-run formal verification for the affected items.
6. Mark `fixed` only when both pass. New defects discovered mid-loop become new findings.

Stop only when every finding is `fixed` or explicitly `deferred` with a one-line reason and follow-up location.

## Output (exact shape, nothing else)
```
Scope: <files/functions/behaviors/artifact>
Source: <path or "conversation" or "scope input">

Specification:
- <item> — source: <where>

Empirical:
- <item>: <command> -> <result> (evidence: <ref>)

Formal:
- <item>: <claim> — <reasoning> — <code refs>

Findings:
- [<severity>] <file>:<line> <defect> — <evidence> — <root cause> — <fixed|deferred: reason>

Fixes:
- <file>:<line> <summary> (tests: <names that fail without the fix>)

Re-verification:
- <command> -> <result>

Open questions:
- <item> — <why unresolved>
```

## Acceptance criteria
- `Scope` and `Source` emitted before verification.
- Important contract items have primary sources, empirical evidence, and code reasoning with refs.
- No hedging vocabulary.
- Every finding has evidence, root cause, and `fixed` or `deferred` with reason.
- Every fix has a new/strengthened test that fails without it (unless the fix is purely a spec/doc correction; state that explicitly).
- Re-verification commands ran after the last fix and passed.
- No suppressions, baselines, snapshot rewrites, dependency bumps, build/test-config edits, or test deletions used to clear failures.
- "No issues" without listed evidence is rejected.
