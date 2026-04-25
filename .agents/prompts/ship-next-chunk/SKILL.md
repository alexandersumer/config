---
name: ship-next-chunk
description: Sync to fresh main, pick the next project chunk from the spec, implement it, and put it on a new branch
argument-hint: "[optional: project goal or focus]"
inputs:
  - name: focus
    label: Project focus
    description: Optional override for the next chunk to tackle. Leave empty to read the spec from the repo or conversation.
    type: string
    required: false
  - name: project_root
    label: Project root path
    description: Optional path that anchors spec resolution in monorepos with multiple parallel plans (e.g. a service directory or a plan subdirectory). Leave empty to infer from the just-merged diff.
    type: string
    required: false
---

The previous branch merged. Reset to a fresh default branch, read the project spec, pick the next chunk, ship it on a new branch (uncommitted).

## Refresh the default branch

Preconditions (stop and report if either fails):
- `git status` is clean.
- `origin` has a default branch (`main` or `master`).

Switch to the default branch. Run `git fetch origin <default-branch> --prune`, then `git reset --hard origin/<default-branch>`. Confirm `git status --porcelain` is empty and `git rev-parse HEAD` matches `git rev-parse origin/<default-branch>`.

## Read the project spec

Compute the locality signal first — the just-merged diff is the strongest clue to which project you are on:
- Merged paths: `git log -1 --name-only --pretty=format: origin/<default-branch>`.
- Previous branch: `git reflog --pretty=%gs | grep -m1 'checkout: moving from' | sed -E 's/.*moving from ([^ ]+) to.*/\1/'`.

Derive the locality prefix set: longest common path prefixes from the merged files plus any project token in the previous branch name (e.g. `cli` from `feat/cli-foo`).

Output: `Locality: <prefixes> (from <signal>)`.

Resolve the spec from the first source that yields one:
1. `focus` input — treat as the spec.
2. `project_root` input — restrict the search below to that path.
3. Planning artifact, scored by locality. Collect candidates:
   - `git ls-files | grep -iE '^(\.plan|\.projects|plan|project|projects|roadmap|todo|tasks|backlog|milestones)(\.md)?$'`
   - `git ls-files 'docs/*' '.plan/*' '.projects/*' '.tasks/*' '*/PLAN.md' '*/ROADMAP.md' '*/TODO.md' '*/tasks.md' 2>/dev/null`

   Score each: +10 path starts with a locality prefix, +5 contents reference a merged path, +3 path or contents contain a previous-branch token, +1 most recently modified. Pick the highest.
4. Conversation context.

Output: `Spec source: <path or "conversation" or "focus input">`.

Stop and ask the user (do not guess) if: the top score is 0, the top two are within 5 points, or the chosen spec contains no merged path and no previous-branch token.

## Pick the next chunk

A chunk qualifies when **all** of these hold:

- It is the next item in the spec's stated sequence, or it unblocks the next stated item.
- It has not shipped (check `git log --oneline -50 origin/<default-branch>` and any open branches via `git branch -a`).
- Its expected diff fits the size band: 1 to ~10 files, roughly 50–500 lines net, one user-visible behavior or one internal capability that another chunk will consume.
- It has a single observable acceptance signal you can name in one sentence (a test that passes, a CLI that produces output X, an endpoint that returns Y).

Examples of qualifying chunks:
- `add JSON output mode to the report command, with golden-file tests`
- `extract the retry policy into a single class and route the three call sites through it`
- `implement the /healthz endpoint with liveness and readiness probes plus integration tests`

Examples that do not qualify (state why and pick a different one):
- `improve error handling` — no observable signal.
- `clean up the codebase` — not in the spec, no boundary.
- `rename one variable` — below the size band.
- `rewrite the auth layer` — above the size band, ship in stages.

If two or more chunks qualify, list them, then pick the one whose acceptance signal is most concrete and announce the pick in one line: `Chunk: <name>. Why: <one sentence tying it to the spec>.`

If the spec file tracks status (checkboxes, status columns, headings like `In progress`), update the chosen chunk's status in that file as part of the diff.

## Implement

Stay on the default branch while reading and exploring. Use file-reading and grep tools liberally to map existing patterns before writing code; explicit reading beats guessing.

Parallelize independent investigations in a single tool batch:
- Spec file(s)
- Recent commits on the default branch (`git log -20 --stat origin/<default-branch>`)
- The modules the chunk will touch
- Existing tests that exercise those modules

Match the surrounding code: same naming, same error-handling style, same test framework, same comment density, same dependency set. Reuse existing helpers and types.

Build the change end to end in one pass:
- Production code for the one behavior.
- Tests that fail under at least one plausible mutation of the new code (off-by-one, swapped args, null vs empty, flipped conditional, wrong exception type).
- User-facing strings, docs, and config the change requires.
- Feature-flag or rollout wrapper if the codebase already uses one for behavioral changes.

Run the local build and test suite. On failure, fix the application code the checker points at. Re-run. Iterate until green. Allowed: edits to application source and tests. Disallowed: `@Suppress`, lint baselines, dependency bumps, build-config edits, test-infra edits.

## Branch and report

Create a branch from the current commit and switch to it: `git switch -c <kebab-case-name>`. Pick the name from the chunk: a verb plus the affected noun, no ticket prefix.

Examples: `add-json-report-output`, `extract-retry-policy`, `implement-healthz-endpoint`.

Leave the changes in the working tree, uncommitted. Do not run `git add`, `git commit`, `git push`, or open a PR.

## Output

Final message uses exactly this shape:

```
Locality: <prefix1>, <prefix2> (from <signal>)
Spec source: <path or "conversation" or "focus input">
Chunk: <name>
Why: <one sentence tying it to the spec>
Branch: <branch-name>

Files:
- <path>
- <path>

Tests:
- <test name>: <mutation it now catches>
- <test name>: <mutation it now catches>

Checks: <command run> -> <pass/fail summary>

Next: run the git-commit-push prompt to ship it.
```

Keep the message under 25 lines. Do not add a preamble, summary, or sign-off.

## Acceptance criteria

- `git rev-parse HEAD` matched `git rev-parse origin/<default-branch>` before any new file was touched.
- Locality line was emitted before spec selection.
- Spec source line was emitted before chunk selection, and the chosen spec passed the verification step (references a just-merged path or a previous-branch-name token).
- Chosen chunk fits the four qualification rules above and was announced in one line.
- Diff touches 1–10 files and stays inside the chunk's stated scope, all under the locality prefix or the supplied `project_root`.
- New tests would fail under at least one named mutation of the new code.
- Local build and test suite passed end to end on the final attempt.
- Current branch is the new kebab-case branch, working tree contains the changes, no commit was created.
- Final message matches the output shape above, ≤25 lines.
