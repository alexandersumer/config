---
name: continue-next
description: Sync to fresh main, pick the next project chunk from the spec, implement it, and put it on a new branch
argument-hint: "[optional: project goal or focus]"
inputs:
  - name: focus
    label: Project focus
    description: Optional override for the next chunk to tackle. Leave empty to read the spec from the repo or conversation.
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

Resolve the spec from the first source that yields one:

1. The `focus` input if non-empty. Treat its text as the spec.
2. A planning artifact in the repo. Run these searches in parallel and read every match:
   - `git ls-files | grep -iE '^(\.plan|\.projects|plan|project|projects|roadmap|todo|tasks|backlog|milestones)(\.md)?$'`
   - `git ls-files 'docs/*' | grep -iE '(plan|project|roadmap|todo|backlog|milestone)'`
   - `git ls-files '.plan/*' '.projects/*' '.tasks/*' 2>/dev/null`
   The most recently modified match (`git log -1 --format=%cI -- <file>`) is authoritative; older matches are background.
3. The preceding conversation: what was just merged, what was promised next.

Output one line: `Spec source: <path or "conversation" or "focus input">`.

If none of the three yields a usable spec, stop and ask the user for one. Do not invent a project.

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

Stay on the default branch while reading and exploring. Use file-reading and grep tools liberally to map existing patterns before writing code; explicit reading beats guessing on Opus 4.7.

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
- Spec source line was emitted before chunk selection.
- Chosen chunk fits the four qualification rules above and was announced in one line.
- Diff touches 1–10 files and stays inside the chunk's stated scope.
- New tests would fail under at least one named mutation of the new code.
- Local build and test suite passed end to end on the final attempt.
- Current branch is the new kebab-case branch, working tree contains the changes, no commit was created.
- Final message matches the output shape above, ≤25 lines.
