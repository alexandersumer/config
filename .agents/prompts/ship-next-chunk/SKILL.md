---
name: ship-next-chunk
description: Sync to the default branch, choose the next artifact-backed chunk, implement it on a new branch, and leave changes uncommitted
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

The previous branch merged. Reset to a fresh default branch, choose the next artifact-backed chunk, implement it on a new branch, and leave changes uncommitted.

Do not satisfy this by picking an easy cleanup or a helper-only slice. The known failure mode is creating a neat branch that does not deliver the next real capability. The chunk must be artifact-backed, reviewable, reachable through real entry points, and locally verified.

Refresh:
- Preconditions: clean working tree and resolvable upstream default branch. If either fails, stop. Do not stash, discard, or guess.
- Determine `<remote>` and `<default-branch>` from upstream HEAD, falling back to common defaults only when needed.
- Switch to default branch.
- Fetch `<remote> <default-branch> --prune`.
- Fast-forward safely. Use `git reset --hard <remote>/<default-branch>` only after confirming the working tree is clean and current branch is the default branch.
- Confirm clean status and `HEAD == <remote>/<default-branch>`.

Discover artifact set:
- Emit locality first: merged paths from `git log -1 --name-only --pretty=format: <remote>/<default-branch>`, previous branch token from reflog if useful, then `Locality: <prefixes> (from <signal>)`.
- Resolve from `artifact`, else `focus`, else `project_root`, else repo artifacts scored by locality: `.plan`, `.projects`, `.tasks`, docs planning dirs, common spec/design/proposal/plan/roadmap/todo files.
- Read companion design/proposal/plan files in the same directory or linked front matter.
- Emit `Artifact source: <path or conversation or artifact input>`.
- Ask, do not guess, if no artifact scores, top candidates are close, or locality evidence is weak.

Pick chunk:
- Next in artifact order or required to unblock the next item.
- Not already implemented in recent default-branch history or existing branches.
- Reviewable: usually 1-10 files / 50-500 net lines, unless artifact needs justify more.
- Delivers one user-visible behavior or internal capability wired into real entry points.
- Has one concrete acceptance signal.
- Reject vague cleanup, one-variable renames, helper/type/test/docs-only chunks, and huge rewrites.
- For infrastructure chunks, reviewability means a usable command/harness, at least one real example, and evidence it would catch a plausible broken case. Do not shrink below autonomous verification capability.

Branch:
- Compute a valid Conventional Commit subject matching `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`.
- Derive branch name as `<type>/<kebab-case-description>`, including scope when useful.
- Create the branch from fresh default before implementation edits.

Implement:
- Read recent default commits, touched modules, existing tests, and remaining artifacts.
- Build production behavior, wiring, tests, required docs/config, and rollout wrapper only if repo convention requires it.
- If existing local tests cannot verify the capability, build the missing targeted path instead of rerunning unrelated green checks.
- New tests must catch a named mutation or realistic regression.
- Forbidden unless artifact requires it: suppressions, baselines, dependency bumps, build-config edits, test-infra edits.

Leave changes uncommitted. Do not add, commit, push, or open a PR.

Final response exactly, under 25 lines:
```text
Locality: <prefixes> (from <signal>)
Artifact source: <path or "conversation" or "artifact input">
Chunk: <name>
Why: <one sentence tying it to the artifact set>
Subject: <Conventional Commit subject>
Branch: <branch-name>

Files:
- <path>

Tests:
- <test name>: <mutation it now catches>

Checks: <command run> -> <pass/fail summary>

Next: run the git-publish prompt to commit, push, and open or update the pull request.
```
