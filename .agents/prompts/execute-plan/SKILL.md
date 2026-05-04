---
name: execute-plan
description: Implement a plan
argument-hint: "[optional: artifact path, plan name, or inline instructions]"
inputs:
  - name: artifact
    label: Planning artifact
    description: Path/name/text for a spec, plan, design, proposal, roadmap, task artifact, or inline instructions. Leave empty to discover the most relevant recent artifact.
    type: string
    required: false
  - name: spec
    label: Planning artifact (legacy alias)
    description: Backward-compatible alias for artifact. Prefer artifact for new usage.
    type: string
    required: false
---

Implement `artifact`, `spec`, `$ARGUMENTS`, or the most relevant discovered planning artifact.

Do not stop at scaffolding, types, TODOs, docs, or unrelated green tests. Deliver reachable behavior through real repo entry points.

Before editing:
- read the artifact set end to end
- read README/CONTRIBUTING, touched modules, nearest tests, and 2-3 sibling features
- emit `Artifact: <path or inline>` and `Canonical patterns: <path> for <aspect>`

Use update_todo. Every task must be an observable behavior or capability with a checkable signal.

Build production code, wiring, tests, and required docs/config together. Tests must catch a named realistic regression. If no existing check can prove the behavior, add the missing targeted check instead of claiming green.

Test infra/build config changes are allowed only when the artifact asks for verification infrastructure. Otherwise no suppressions, baselines, dependency bumps, build config edits, skipped tests, or fake TODO placeholders.

Run targeted checks, then broader checks when available. Re-read the artifact and account for every requirement as implemented or deferred.

Final under 25 lines:
```text
Artifact: <path or inline>
Implemented:
- <behavior>: <files> — `<command>` -> <result>
Checks: `<command>` -> <result> or `not run — <reason>`
Deferred:
- <item or None>: <reason>
Next: review-branch, then git-publish.
```
