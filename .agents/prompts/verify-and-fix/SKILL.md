---
name: verify-and-fix
description: Perform adversarial formal and empirical verification of scoped behavior, then fix only proven defects
argument-hint: "[optional: scope, file, function, behavior, or planning artifact]"
inputs:
  - name: scope
    label: Verification scope
    description: Path, function, behavior, or planning artifact to verify. Leave empty to infer from conversation, recent changes, or relevant artifact.
    type: string
    required: false
---

You are not doing a casual check. You are trying to disprove the scoped behavior before you trust it.

Verification has two independent burdens of proof:
1. **Empirical:** executable evidence demonstrates the behavior through real code paths.
2. **Formal:** source-derived reasoning explains why the behavior follows from the implementation for the relevant state/input space.

A claim is verified only when both burdens are met. If either burden is missing, report `unverified`; do not translate uncertainty into confidence.

## Resolve scope and standard of proof
First source that yields a meaningful scope: `scope` input → recent changes (`git status`, `git diff base...HEAD`) → relevant planning artifact → conversation. Emit `Scope:`, `Source:`, and `Verification standard:` before inspecting correctness.

If the scope is ambiguous, too broad to verify deeply, or too small to matter, ask a narrowing question. Prefer a smaller scope with hard evidence over a broad scope with weak statements.

Set the verification standard explicitly:
- `exhaustive-by-reasoning` for small finite logic, pure functions, parsers with bounded grammar, or state machines where cases can be enumerated.
- `representative-adversarial` for large systems, integrations, UI flows, distributed behavior, performance, or anything with a large input/state space.

## Build the specification from primary sources
Before evaluating code, list the contract items that must be true. Use primary sources: user request, planning artifact, public API docs, schemas, type declarations, tests that encode intended behavior, existing callers, migration notes, or compatibility requirements.

For each item, record:
- **Source:** exact file/function/test/doc/conversation reference.
- **Observable behavior:** what an external caller/user/system can observe.
- **Invariants:** preconditions, postconditions, state transitions, ordering, idempotency, isolation, authorization, persistence, and error semantics.
- **Boundaries:** null/empty/zero/one/many, min/max, malformed input, unicode/encoding, time zones/time travel, cancellation, concurrency, retries, partial failure, large inputs, adversarial inputs.
- **Consumers/producers:** upstream and downstream API, wire, schema, storage, or UI dependencies.

Items without a primary source are `open questions`, not assumptions. Do not verify against guessed intent.

## Formal verification pass
For every in-scope contract item, perform source-level reasoning that would survive review by a skeptical maintainer:
- Trace control flow, data flow, state mutation, side effects, and error flow from entry point to observable result.
- Enumerate cases relevant to the verification standard. For `exhaustive-by-reasoning`, cover every meaningful branch/state combination. For `representative-adversarial`, explain why chosen partitions cover the risk surface.
- Check preconditions and postconditions at component boundaries, not just inside the edited function.
- Cross-check types, schemas, serialization formats, permissions, feature flags, migrations, and consumers against the implementation.
- Identify where the reasoning depends on external behavior, timing, environment, nondeterminism, or undocumented assumptions.

Record formal claims as falsifiable statements: `Formal: <item> — claim: <specific claim> — proof sketch: <case/control/data reasoning> — refs: <file:line...> — gaps: <none|gap>`.

Reading code is not enough. The formal pass must produce a proof sketch or a gap.

## Empirical verification pass
For every in-scope contract item, produce executable evidence through the realest available path:
- Discover and use the repo's canonical commands from README/package/build files before inventing commands.
- Prefer integration or public-entry tests over private-function tests. Use mocks only at true external boundaries; never mock the behavior being verified and then claim it works.
- Add or strengthen targeted tests for missing boundary/failure/adversarial cases before claiming coverage.
- For every new or strengthened defect test, demonstrate it is meaningful: it must fail on the defective implementation, fail under a realistic mutation, or be justified as impossible to pre-fail because the defect is environmental/spec-only.
- Run the narrowest relevant command first, then the broader regression command needed for confidence.
- Capture enough output to establish command, result, and covered behavior. Do not merely say "tests pass".

Record empirical evidence as: `Empirical: <item> — command: <command> — result: <pass|fail|blocked> — evidence: <test name/log excerpt/ref> — coverage: <cases exercised> — gaps: <none|gap>`.

A passing unrelated suite is not evidence for a contract item. A test name without the behavior it covers is not evidence.

## Adversarial verification requirements
Actively look for reasons the implementation could be wrong:
- Mutation thinking: name at least one plausible bug per important behavior (flipped conditional, off-by-one, stale cache, missed await, wrong exception, swallowed error, race, schema drift, auth bypass, timezone/encoding bug) and identify what evidence would catch it.
- Negative controls: include at least one invalid/error/edge path when the behavior has meaningful failure modes.
- Cross-boundary checks: if data crosses process, network, storage, queue, API, or UI boundaries, verify both producer and consumer expectations.
- Regression risk: verify unchanged behavior that could plausibly be affected by the fix.

If you cannot perform one of these checks, record it as a verification gap with the reason and the residual risk.

## Findings
A finding is any proven defect, spec/code disagreement, missing in-scope test evidence, unverified high-risk assumption, or verification gap that prevents confidence.

For each finding, include:
- Severity: `critical|high|medium|low`.
- Location and contract item affected.
- Evidence: empirical failure, formal contradiction, missing proof, or missing executable coverage.
- Root cause, not just symptom.
- Fix status: `fixed`, `deferred: <reason>`, or `blocked: <reason>`.

Group symptoms with the same root cause under one finding. Do not bury missing evidence in prose; list it as a finding or gap.

## Fix loop
For each finding, highest severity first:
1. Make the smallest correct root-cause fix. Avoid band-aids, silent fallbacks, broad `try/catch`, sleeps, retries without cause, or defensive code that hides the defect.
2. Preserve intended public behavior and compatibility unless the primary source says otherwise.
3. Add or strengthen a test that detects the defect or guards the verified contract. Prefer a test that would fail without the fix.
4. Re-run empirical verification for the affected item and a regression command covering adjacent scope.
5. Re-run the formal proof sketch for the affected item and update refs/gaps.
6. If the fix reveals a new defect, add a new finding rather than expanding scope silently.

Mark `fixed` only after formal and empirical verification both pass. If no code changes are needed, say why the evidence proves that.

## Forbidden verification shortcuts
These are defects in the verification itself:
- Hedging or vibe words: "looks fine", "seems okay", "should work", "appears to", "likely", "probably", "I think".
- Correctness claims without both executable evidence and source-level proof.
- Claiming coverage without naming the exact behavior/case the test exercises.
- Treating compile/typecheck/lint success as behavioral verification.
- Trusting mocks of the system under test as evidence that the system works.
- Ignoring failing, flaky, skipped, quarantined, or TODO tests that intersect the scope.
- Suppressions, baselines, snapshot rewrites, dependency bumps, build/test config changes, warning disables, test deletion, weakened assertions, or moving cases out of scope to get green.
- Rewriting code so the failing test no longer applies instead of fixing the defect.
- Saying "no issues" while any important contract item has `gaps`.

## Output shape
Use this exact structure. Be concise, but every verified claim needs evidence.

```
Scope: <files/functions/behaviors/artifact>
Source: <scope input|path|conversation|branch diff>
Verification standard: <exhaustive-by-reasoning|representative-adversarial> — <why appropriate>

Specification:
- <item id>: <contract item> — source: <primary source> — observable: <behavior> — boundaries/risks: <list>

Formal verification:
- <item id>: claim: <specific claim> — proof sketch: <case/control/data reasoning> — refs: <file:line...> — gaps: <none|gap>

Empirical verification:
- <item id>: command: <command> — result: <pass|fail|blocked> — evidence: <test/log/ref> — coverage: <cases> — gaps: <none|gap>

Adversarial checks:
- <item id>: plausible bug: <mutation/failure mode> — caught by: <test/proof/check> — residual risk: <none|risk>

Findings:
- [<severity>] <item id> <file:line> <defect/gap> — evidence: <evidence> — root cause: <cause> — status: <fixed|deferred: reason|blocked: reason>

Fixes:
- <file:line> <summary> — verification added/updated: <test/check> — would fail without fix: <yes|no + why>

Re-verification:
- <command> -> <result> — scope covered: <items>

Open questions / residual risk:
- <item or none> — <why unresolved and what would resolve it>
```

## Acceptance criteria
- Scope, source, and verification standard are emitted before correctness claims.
- Each important contract item has a primary source, a formal proof sketch, empirical evidence, adversarial consideration, and explicit gaps.
- Every empirical claim names the command, result, test/log evidence, and behavior covered.
- Every formal claim includes concrete code references and case/control/data reasoning.
- Every finding has severity, evidence, root cause, and status.
- Every fix has a new or strengthened verification check unless explicitly justified.
- Re-verification ran after the last fix and covers the changed behavior plus plausible regressions.
- No forbidden shortcuts were used.
- `No issues found` is valid only when every contract item has `gaps: none` in both formal and empirical verification.
