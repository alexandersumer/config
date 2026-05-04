---
name: ship-next-chunk
description: Ship the next planned chunk
argument-hint: "[optional: project, plan, spec, artifact path, or inline artifact]"
inputs:
  - name: artifact
    label: Project, plan, spec, or artifact
    description: Optional project name, plan/spec/design/proposal/roadmap/task path or text, or focus for the next chunk. Leave empty to infer from repo/context.
    type: string
    required: false
  - name: focus
    label: Project/focus (legacy alias)
    description: Backward-compatible alias for artifact. Prefer artifact for new usage.
    type: string
    required: false
  - name: project_root
    label: Project root path
    description: Optional path that anchors artifact discovery in repos with multiple parallel plans. Leave empty to infer from the just-merged diff.
    type: string
    required: false
---

Start from fresh default, choose the next artifact-backed chunk, implement it on a new branch, leave it uncommitted.

Do not pick cleanup or helper-only work. Ship the next real behavior/capability through real entry points with local verification.

Refresh:
- require clean working tree and resolvable upstream default
- switch to default branch
- run `git fetch --prune <remote> <default>`
- fast-forward to `<remote>/<default>`; stop if that cannot be done cleanly

Discover:
- emit `Locality: <prefixes> (from <signal>)`
- use `artifact`, `focus`, `project_root`, last-merge paths, reflog token, or repo planning files
- read the artifact set and companions
- emit `Artifact source: <path|conversation|artifact input>`
- ask if candidates tie or locality is weak

Pick a chunk that is:
- next or unblocking in the artifact
- not already implemented
- reviewable unless the artifact requires more
- wired into real entry points
- backed by one acceptance signal
- not docs/tests/types/helpers-only unless paired with behavior

For infra chunks, the unit of work is a usable local command/harness plus one real example and one plausible broken case it catches.

Create `<type>/<kebab-description>` from fresh default using a valid Conventional Commit subject.

Implement code, wiring, tests, and required docs/config. Tests must catch a named regression. If existing checks cannot prove the behavior, add the missing targeted path.

No suppressions, baselines, dependency bumps, skipped tests, or build/test infra edits unless the artifact requires them.

Run targeted checks and broader checks when available. Do not commit, push, or open a PR.

Final exactly:
```text
Locality: <prefixes> (from <signal>)
Artifact source: <path or "conversation" or "artifact input">
Chunk: <name>
Why: <artifact reason>
Subject: <Conventional Commit subject>
Branch: <branch-name>
Files:
- <path>
Tests:
- <test name>: <mutation it catches>
Checks: <command> -> <result> or not run — <reason>
Next: run git-publish.
```
