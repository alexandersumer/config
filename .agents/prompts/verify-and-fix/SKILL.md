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

Try to disprove the scoped behavior before trusting it. Verification requires both source-level proof and executable evidence. If either is missing, the claim is unverified.

Do not satisfy this by running existing green tests and saying the behavior looks fine. The known failure mode is confidence laundering: compile success, unrelated tests, mocks of the system under test, or prose reasoning without a real failure detector. If the repo lacks a check that could catch the bug class, build the check or report the gap.

Resolve scope from `scope`, else `$ARGUMENTS`, else recent branch diff, else relevant planning artifact, else conversation. Emit before correctness claims:
- `Scope: <files/functions/behaviors>`
- `Source: <input|diff|artifact|conversation>`
- `Verification standard: <exhaustive-by-reasoning|representative-adversarial> — <why>`

Build the specification from primary sources only: user request, artifact, public API docs, schemas, type declarations, tests that encode intended behavior, callers, migrations, compatibility requirements. Anything without a primary source is an open question.

For each important contract item, record:
- observable behavior
- invariants and boundaries
- consumers/producers across API, storage, UI, network, queue, or process boundaries
- plausible bug that would violate it

Formal pass:
- Trace control flow, data flow, state mutation, side effects, and error flow from entry point to observable result.
- Enumerate cases. Exhaust small finite logic; partition large systems adversarially.
- Check boundary contracts, schemas, permissions, flags, serialization, migration, and consumers.
- Produce falsifiable proof sketches with code refs and explicit gaps.

Empirical pass:
- Use canonical repo commands discovered from README/build files.
- Prefer integration or public-entry tests. Mock only true external boundaries.
- Add or strengthen targeted tests when current checks cannot catch the named bug.
- For each new/changed test, state whether it would fail without the fix or what mutation it catches.
- Run narrow checks first, then broader regression checks.

Fix loop:
- Fix only proven defects or gaps that block verification.
- Make the smallest root-cause fix; no band-aids, sleeps, broad catches, silent fallbacks, suppressions, baselines, dependency bumps, skipped tests, deleted tests, or weakened assertions.
- Re-run empirical checks and update the formal proof after each fix.

Output:
```text
Scope: <files/functions/behaviors/artifact>
Source: <scope input|path|conversation|branch diff>
Verification standard: <standard> — <why>

Specification:
- <id>: <contract> — source: <primary source> — observable: <behavior> — risks: <boundaries/bugs>

Formal verification:
- <id>: claim: <specific claim> — proof: <case/control/data reasoning> — refs: <file:line...> — gaps: <none|gap>

Empirical verification:
- <id>: command: <command> — result: <pass|fail|blocked> — evidence: <test/log/ref> — coverage: <cases> — gaps: <none|gap>

Adversarial checks:
- <id>: plausible bug: <mutation/failure mode> — caught by: <test/proof/check> — residual risk: <none|risk>

Findings:
- [<severity>] <id> <file:line> <defect/gap> — evidence: <evidence> — root cause: <cause> — status: <fixed|deferred: reason|blocked: reason>

Fixes:
- <file:line> <summary> — verification: <test/check> — would fail without fix: <yes|no + why>

Re-verification:
- <command> -> <result> — scope covered: <items>

Open questions / residual risk:
- <item or none> — <what would resolve it>
```

`No issues found` is valid only when every important contract item has `gaps: none` in both formal and empirical verification.
