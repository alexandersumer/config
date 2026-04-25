---
name: ship-next-chunk
description: Sync to the fresh default branch, pick the next artifact-backed chunk, implement it, and put it on a new branch
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

The previous branch merged. Reset to a fresh default branch, read the relevant artifact set, pick the next chunk, implement it on a new branch, and leave it uncommitted.

## Refresh
Preconditions: clean working tree; upstream remote has a resolvable default branch. If either fails, stop and report instead of stashing, discarding, or guessing.

Determine `<remote>` and `<default-branch>` from upstream HEAD, falling back to common default names only if needed. Switch to default branch. Run `git fetch <remote> <default-branch> --prune`, then fast-forward to `<remote>/<default-branch>` when safe. Use `git reset --hard <remote>/<default-branch>` only after confirming the working tree is clean and the current branch is the default branch. Confirm clean status and `HEAD == <remote>/<default-branch>`.

## Discover artifact set
Emit locality before artifact selection:
- Merged paths: `git log -1 --name-only --pretty=format: <remote>/<default-branch>`.
- Previous branch token from reflog if available.
- Output `Locality: <prefixes> (from <signal>)`.

Resolve from first source that works:
1. `artifact` input as project name, artifact path/text, or discovery focus.
2. Legacy `focus` input the same way, if `artifact` is empty.
3. `project_root` as search restriction.
4. Repo artifacts scored by locality: `.plan`, `.projects`, `.tasks`, docs planning dirs, common `SPEC/DESIGN/PROPOSAL/PLAN/ROADMAP/TODO/tasks` filenames. Score path locality, references to merged paths, previous-branch tokens, implementation-plan role, recency. Read companion design/proposal/plan files in same dir or linked front matter.
5. Conversation/repo context when nothing is specified.

Emit `Artifact source: <path or "conversation" or "artifact input">`. Ask, do not guess, if no artifact scores, top two are close, or chosen set lacks locality evidence.

## Pick chunk
A chunk qualifies only if:
- It is next in the artifact set or unblocks the next item.
- It has not shipped (`git log -50 <remote>/<default-branch>`, `git branch -a`).
- Expected diff is reviewable, usually about 1–10 files / 50–500 net lines; use artifact needs over numeric targets.
- It delivers one user-visible behavior or internal capability wired into real entry points.
- It has one concrete acceptance signal.

Reject vague cleanup, one-variable renames, helper/type/test/docs-only nibbles, and huge rewrites. If multiple qualify, pick the one with the clearest acceptance signal and announce: `Chunk: <name>. Why: <artifact-set reason>`.

If artifacts track status, update the chosen chunk status in the relevant artifact.

## Implement
Stay on default branch while reading. In parallel read artifact files, recent default-branch commits, touched modules, and existing tests.

Match surrounding code: naming, error handling, test framework, comment density, dependencies. Build end to end: production behavior, wiring, tests catching at least one named mutation, required docs/config, rollout wrapper if the codebase already uses one for behavioral changes.

Run local build/tests; fix application/test code until green. Disallowed: suppressions, lint baselines, dependency bumps, build-config edits, test-infra edits.

## Branch and output
Create `git switch -c <kebab-case-name>` from current commit. Branch name must derive from a Conventional Commits subject for the chunk:
- Compute a Conventional Commit subject `<type>(<optional-scope>): <description>` matching `^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`.
- Derive the branch as `<type>/<kebab-case-description>` (include `<scope>` when useful, e.g. `feat/auth-add-pkce`).
- No ticket prefix, no marketing language, no emojis.

Leave changes uncommitted; do not add/commit/push/open PR. Downstream `create-pull-request` can reuse this subject as the commit message and PR title.

Final response exactly:
```
Locality: <prefixes> (from <signal>)
Artifact source: <path or "conversation" or "artifact input">
Chunk: <name>
Why: <one sentence tying it to the artifact set>
Subject: <Conventional Commit subject for the chunk>
Branch: <branch-name>

Files:
- <path>

Tests:
- <test name>: <mutation it now catches>

Checks: <command run> -> <pass/fail summary>

Next: run the create-pull-request prompt to ship it.
```
Under 25 lines. No preamble, summary, or sign-off.

## Acceptance criteria
- HEAD matched `<remote>/<default-branch>` before edits.
- Locality emitted before artifact selection; artifact source before chunk selection.
- Chosen artifact set passed locality verification.
- Chunk fits qualification rules and scope.
- Diff stays within locality/project_root and touches 1–10 files unless artifact justifies otherwise.
- New tests catch named mutation.
- Final build/tests pass.
- Current branch is a new kebab-case branch derived from a valid Conventional Commit subject; working tree has changes; no commit.
- Final message matches output shape.
