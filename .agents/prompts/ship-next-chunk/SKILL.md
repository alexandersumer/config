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

Start from fresh default, pick the next artifact-backed chunk, implement it on a new branch, leave it uncommitted.

Do not pick easy cleanup. The chunk must deliver the next real capability through real entry points with local verification.

Refresh:
- require clean working tree and resolvable upstream default
- switch to default branch
- fetch `<remote> <default> --prune`
- fast-forward or clean reset to `<remote>/<default>` only when safe
- confirm clean status and matching HEAD

Discover:
- emit `Locality: <prefixes> (from <signal>)` from last merge paths, reflog token, `artifact`, `focus`, or `project_root`
- resolve artifact set from inputs or repo planning files
- read companion artifacts
- emit `Artifact source: <path|conversation|artifact input>`
- ask if locality is weak or candidates tie

Pick chunk:
- next or unblocking artifact item
- not already implemented
- reviewable size unless artifact requires more
- one behavior/capability wired into real entry points
- one acceptance signal
- not docs/tests/types/helpers-only unless paired with behavior

For infra chunks, do not shrink below a usable local command/harness with one real example and a plausible broken case it catches.

Branch:
- compute valid Conventional Commit subject
- create `<type>/<kebab-description>` from fresh default

Implement:
- read touched modules, tests, recent default commits, remaining artifact context
- build code, wiring, tests, required docs/config
- tests catch a named realistic regression
- if existing checks cannot prove the behavior, add the missing targeted path
- no suppressions, baselines, dependency bumps, skipped tests, or build/test infra edits unless artifact requires them

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
Checks: <command> -> <result>
Next: run git-publish.
```
