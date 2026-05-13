---
name: ship-next-chunk
description: Ship the next planned chunk
register_cmd: true
---

Start from fresh default, choose the next artifact-backed chunk, implement it on a new branch, leave it uncommitted. If the user did not explicitly request shipping a chunk, ask before any git write.

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

Pick the next chunk like a maintainer, not a task-list parser. The right chunk is the next artifact-backed piece of observable behavior that can be reviewed independently and proven through a real entry point.

Do not choose cleanup, scaffolding, test-only, type-only, helper-only, or docs-only work unless it is inseparable from shipped behavior. If the artifact's next chunk is stale, already implemented, too broad, or not locally provable, say so and choose the next reachable slice. If candidates tie or locality is weak, ask before branching.

For infra chunks, the unit of work is a usable local command/harness plus one real example and one plausible broken case it catches.

Create `<type>/<kebab-description>` from fresh default using a valid Conventional Commit subject.

Implement code, wiring, tests, and required docs/config. Tests must catch a named regression. If existing checks cannot prove the behavior, add the missing targeted path.

No suppressions, baselines, dependency bumps, skipped tests, or build/test infra edits unless the artifact requires them.

Run targeted checks and broader checks when available. Evidence before claims: no fixed, complete, ready, or passing language without fresh proof in this turn. Do not commit, push, or open a PR.

Final response should be short: locality, artifact source, chosen chunk, why that slice was right, branch, changed files, tests with the bug they catch, checks, and the next command. No padded shipping report.
