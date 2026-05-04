---
name: execute-plan
description: Implement a planning artifact with repo patterns, tests, and verification
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

Implement the artifact as working software. Not a sketch, not a docs-only pass, not types without reachability, not a tiny safe nibble when the artifact asks for a real capability.

Do not satisfy this by doing the smallest locally defensible edit. The known failure mode is stopping after scaffolding, rerunning unrelated green tests, or claiming progress without a real entry point. The deliverable is behavior that works through the repo's actual wiring and can be verified locally.

Resolve the artifact:
- Use `artifact`, else `spec`, else `$ARGUMENTS`.
- If empty, discover repo planning artifacts in `.plan`, `.projects`, `.tasks`, docs planning dirs, and common spec/design/proposal/plan/roadmap/todo files. Prefer an implementation plan with a companion primary artifact.
- If input is a file, read it. If directory, read planning artifacts inside it. If bare name, resolve across planning locations. Otherwise treat it as inline instructions.
- Read the artifact set end to end before coding. Use the primary artifact for intent and the plan for order.
- Emit `Artifact: <path or inline>`. If no artifact or acceptance can be resolved, ask one batched question and stop.

Learn the implementation bar before editing:
- Read README/CONTRIBUTING/repo-local instructions, touched modules, nearest tests, and 2-3 sibling features with the same shape.
- If the artifact asks for local verification infrastructure, e2e support, or harness work and this repo lacks it, inspect known-good examples available on disk and bring back the smallest durable pattern that fits this repo.
- Emit `Canonical patterns: <path> for <aspect>` lines before editing.

Use update_todo. Each task must be one observable behavior or capability with a checkable acceptance signal. Re-read the artifact after planning; every requirement must be implemented or listed as deferred with a reason.

Implementation rules:
- Build production code, wiring, tests, and required docs/config together.
- New code must be reachable from real entry points.
- Tests must catch at least one named plausible regression or mutation for each important behavior.
- If existing tests cannot prove the behavior, create the missing targeted verification path instead of pretending green is evidence.
- For verification-infra artifacts, test harness/build config changes are in scope. For ordinary product changes, do not touch test infra, dependency versions, baselines, or build config unless the artifact requires it.
- Do not add suppressions, skipped tests, TODO placeholders, broad refactors, or fake rollout wrappers.

Verification before final response:
- Run the smallest meaningful check for each implemented behavior, then the broader local suite when available.
- Re-read the artifact and confirm every requirement is implemented or deferred.
- Confirm the diff stays inside artifact scope and clears the substantial-work bar.

Final response under 35 lines:
```text
Artifact: <path or inline>
Canonical patterns:
- <path> for <aspect>

Implemented:
- <behavior>: <files> — verified by `<command>` -> <result>

Diff size: <N files, ~M lines net>
Checks: `<command>` -> <pass summary>

Deferred:
- <item or None>: <reason>

Next: review-branch, then git-publish.
```
