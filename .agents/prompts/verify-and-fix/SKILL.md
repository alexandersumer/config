---
name: verify-and-fix
description: Verify behavior and fix proven defects
argument-hint: "[optional: scope, file, function, behavior, or planning artifact]"
inputs:
  - name: scope
    label: Verification scope
    description: Path, function, behavior, or planning artifact to verify. Leave empty to infer from conversation, recent changes, or relevant artifact.
    type: string
    required: false
---

Verify `scope`, `$ARGUMENTS`, recent changes, relevant artifact, or conversation behavior.

Do not call existing green tests verification. A claim is verified only with both source-level proof and executable evidence through a real path.

Emit first:
- `Scope: <scope>`
- `Source: <input|diff|artifact|conversation>`
- `Standard: <exhaustive-by-reasoning|representative-adversarial> — <why>`

Build the spec from primary sources only: request, artifact, public API docs, schemas, types, intended-behavior tests, callers, migrations. No source means open question.

For each important contract:
- state observable behavior and risks
- trace control/data/error flow with code refs
- name a plausible bug
- run or add a check that would catch it

If current checks cannot catch the bug class, build the missing check or report the gap. Do not use mocks of the system under test as proof.

Fix only proven defects or verification gaps. No suppressions, baselines, dependency bumps, skipped/deleted/weakened tests, sleeps, broad catches, or silent fallbacks.

Final:
```text
Scope: <scope>
Source: <source>
Standard: <standard>
Spec:
- <id>: <contract> — source: <source> — risks: <bugs/boundaries>
Proof:
- <id>: <claim> — refs: <file:line> — gaps: <none|gap>
Evidence:
- <id>: `<command>` -> <result> — covers: <cases> — gaps: <none|gap>
Findings:
- [<severity>] <defect/gap> — status: <fixed|deferred|blocked>
Fixes:
- <file:line> <summary> — check: <test/command>
Residual risk:
- <none or item>
```

`No issues found` is valid only when every important item has no proof or evidence gap.
