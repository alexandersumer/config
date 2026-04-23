---
name: continue-next
description: Sync to fresh main, then pick up the next substantial chunk of project work on a new branch
argument-hint: "[optional: project goal or focus]"
inputs:
  - name: focus
    label: Project focus
    description: Optional description of the active project or next chunk to tackle. Leave empty to infer from conversation context.
    type: string
    required: false
---

The previous branch has been merged. Reset to a fresh default branch and continue the project with the next chunk of work.

$ARGUMENTS

## Refresh main

Preconditions (stop and report if either fails):
- Working tree has no uncommitted changes. If it does, tell the user to commit or stash first.
- A default branch (`main` or `master`) exists on `origin`.

Determine the default branch. Switch to it. Run `git fetch origin <default-branch> --prune`, then `git reset --hard origin/<default-branch>` so the local default branch matches origin exactly. Confirm `git status` is clean and `git log -1` shows the latest origin commit.

## Identify the project spec and next chunk

Resolve the project spec in this order. Stop at the first source that yields a usable spec:
1. `$ARGUMENTS` if provided.
2. A planning file or directory in the repo. Check `.plan`, `.plan/`, `.projects`, `.projects/`, `PROJECT.md`, `PROJECTS.md`, `PLAN.md`, `ROADMAP.md`, `TODO.md`, `docs/plan*`, `docs/project*`, `docs/roadmap*`, and any equivalent the repo uses. Use `git ls-files` and a case-insensitive search to find them. Read the most recent or most specific file.
3. The preceding conversation context (what was just merged, what remains).

State the spec source you used in one line before proceeding.

From the spec, identify what just shipped and what is still open. Pick the next chunk that is:
- Substantial enough to move the project meaningfully closer to completion (not a one-line tweak).
- Self-contained enough to ship as a single PR.
- Logically next given what just merged (do not skip prerequisites, do not duplicate work already in flight).

If the next chunk is ambiguous, list the candidate chunks you considered and pick one with a one-sentence justification before proceeding.

If a planning file exists and tracks task status, mark the chosen chunk as in progress in that file as part of the change.

## Do the work

Stay on the default branch while exploring; only create the new branch once you are ready to commit (see below).

Read the relevant files first to learn the existing patterns. Match those patterns: naming, error handling, layering, test style, comment density. Do not invent new abstractions or dependencies when existing ones fit.

Implement the change end to end:
- Production code, tests, and any user-facing strings or docs that the change requires.
- Cover real failure modes with tests that would fail under a plausible mutation of the new code.
- Behavioral changes that affect users go behind the project's existing feature-flag or rollout mechanism if one is in use.

After implementing, run the local build and test suite. Fix failures by correcting the code the checker points at, not by suppressing checks, bumping versions, or editing baselines. Iterate until green.

## Branch and hand off

Once the work is complete and checks are green, create a short descriptive branch from the current commit (kebab-case, names the change, no ticket prefix) and switch to it. Do not commit, push, or open a PR; leave that to the user or a follow-up prompt.

Acceptance criteria:
- Local default branch matches `origin/<default-branch>` exactly before any new work begins.
- The chosen chunk is named explicitly with a one-sentence justification tying it to the project's remaining work.
- Diff is scoped to the chosen chunk; no drive-by changes.
- New code reads like the surrounding code and is covered by tests that would catch a plausible mutation.
- Local build and test suite pass end to end.
- A new branch exists for the work, with the changes present in the working tree, uncommitted.
- Final report lists: chunk chosen, files touched, tests added or updated, branch name, and what the user should do next.
