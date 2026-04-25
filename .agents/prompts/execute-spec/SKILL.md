---
name: execute-spec
description: Execute a planning artifact end-to-end as substantial production-grade work, with canonical patterns and robust verification
argument-hint: "[optional: artifact path, plan name, or inline instructions]"
inputs:
  - name: artifact
    label: Planning artifact
    description: Path to a spec, plan, design, proposal, roadmap, or task artifact; a bare artifact name to resolve under repo planning directories; or inline instructions. Leave empty to discover the most relevant recent planning artifact.
    type: string
    required: false
  - name: spec
    label: Planning artifact (legacy alias)
    description: Backward-compatible alias for artifact. Prefer artifact for new usage.
    type: string
    required: false
---

<intent>
Deliver the planning artifact below as substantial, working software — not a sketch, not a docs pass, not a stub. The artifact may be a spec, plan, design, proposal, roadmap, task list, or inline implementation instructions. Production code, real tests, real wiring, full verification, all green.
</intent>

<artifact>
Use the `artifact` input when provided; otherwise use the legacy `spec` input; otherwise use $ARGUMENTS.
</artifact>

<scope_floor>
Do not nibble. Ship the artifact's intended scope in one pass; if the artifact set is genuinely too large, ship the next coherent vertical slice (real production code wired into real entry points, exercised by real tests) and name what was deferred.

Disqualified as substantial: docs/comments-only, types/interfaces-only, TODOs-only, one trivial helper, renames, reformatting. Size signal: typically 5+ files, hundreds of lines net, multiple non-trivial tests, one end-to-end behavior demonstrated by a command you ran. Use judgement; do not pad.
</scope_floor>

<resolve_artifact>
If `<artifact>` is empty, discover the implementation artifact:
1. List repo-native planning artifacts with broad patterns, including `.plan/*`, `.projects/*`, `docs/specs/*`, `docs/design/*`, `docs/plans/*`, `docs/rfcs/*`, `*/PLAN.md`, `*/plan.md`, `*/SPEC.md`, `*/spec.md`, `*/DESIGN.md`, `*/design.md`, `*/PROPOSAL.md`, `*/proposal.md`, `*/ROADMAP.md`, `*/roadmap.md`, `*/TODO.md`, `*/todo.md`, and `*/tasks.md`.
2. Prefer an implementation plan when one clearly exists for the current topic or most recent planning work; otherwise choose the most relevant recent artifact by path, recency, and content.
3. Print `Artifact: <path>`.
4. If none exist, stop and ask the user for an artifact or inline instructions.

If `<artifact>` is a file path, read it. If it is a directory path, read the repo-native planning artifacts inside it, preferring implementation plans before companion designs/proposals. If it is a bare name (e.g. `auth-rotation`), look it up across the repo's planning artifact directories and common planning filenames (with and without `.md`). Otherwise treat it as inline instructions.

Read the artifact set end to end before any other action. If one artifact links to a companion artifact (for example a plan references a design, or a design references a plan), read that companion too. If both a primary artifact and a plan exist, use the primary artifact for intent/scope and the plan for execution order. If the combined artifact set is ambiguous on scope, intent, ordering, or acceptance, ask one batched clarifying question and stop. Do not guess.
</resolve_artifact>

<learn_canonical_patterns>
Before writing code, learn the patterns. Run these reads in one parallel batch:
- The target repo's `README.md`, `CONTRIBUTING.md`, and repo-local agent instruction files such as `AGENTS.md` if present.
- The directories the artifact touches, plus their nearest existing tests.
- Two or three sibling features in the same repo that solve a similar shape of problem, end to end.
- One or two reference repos (prior art on your machine or accessible via tooling) whose layout and language match the target. Grep for the same problem shape (CLI subcommand, HTTP handler, retry policy, migration, worker, cache, etc.). Extract: naming, layering, error handling, test framework, fixture style, dependency injection, comment density, logging, telemetry, feature-flagging.

Output: `Canonical patterns: <repo>:<path> for <aspect>` lines, one per pattern you will mirror.
</learn_canonical_patterns>

<plan>
Decompose the artifact set into an ordered task list using the update_todo tool. If the artifact is already a plan, preserve its intended order unless repo evidence shows a dependency issue. Each task is one observable behavior with a checkable acceptance signal (a test, a CLI output, an endpoint response, a log line, a measurable metric). Mark tasks complete only after their acceptance signal has been demonstrated, not when the file is saved.

Re-read the artifact set at the end of <plan>. Every requirement must appear in the task list or in an explicit deferred-slice list with a stated reason.
</plan>

<implement>
Work through every task. Do not stop at the first plausible stopping point.

For each task:
- Build production code, tests, wiring, docs, and config in one pass. Wire new code into the real entry points (CLI, server, worker, public API) — code that exists but is not reachable does not count.
- Mirror the canonical patterns identified above. Reuse existing helpers, types, fixtures, and abstractions. Do not introduce new dependencies, frameworks, or layers unless the artifact requires them.
- Tests must fail under at least one named mutation of the new code (off-by-one, swapped args, null vs empty, flipped conditional, wrong exception type, dropped side effect). Note the mutation in the test name or a comment. Cover the happy path, at least one failure path, and at least one edge case per behavior.
- Run the full local check suite after each task. Fix the application code the checker points at. Disallowed: per-site warning suppressions, lint/checker baselines, dependency bumps, build-config edits, test-infra edits, skipped or disabled tests, `TODO: implement` placeholders in shipped paths.

Parallelize independent reads and edits in single tool batches. Do not narrate intermediate steps; just do the work.
</implement>

<verify>
Final verification gate, all must pass before you write the report:
- Full local build and test suite green end to end. Capture the exact command and a one-line pass summary.
- Every acceptance signal demonstrated by an actual run (test name + result, CLI invocation + output, HTTP call + response). Not "this should work" — show it worked.
- Artifact set re-read top to bottom; every requirement maps to a shipped file or test, or appears in `Deferred:` with a one-line reason.
- Diff stays inside the artifact's scope; incidental refactors only when required for the change to compile or pass tests.
- The diff clears the <scope_floor>. If it does not, return to <implement>.
</verify>

<acceptance_criteria>
- Artifact source line emitted before any file was written.
- Canonical-patterns lines emitted before any file was written, citing concrete repo paths.
- Every artifact requirement maps to shipped code or a `Deferred:` entry with reason; nothing dropped silently.
- New code is reachable from real entry points, not orphaned.
- Tests cover happy path, failure path, and edge case per behavior; each new test would fail under a named mutation.
- Full local check suite passed end to end on the final attempt with no disallowed suppressions.
- Diff clears the <scope_floor> (substantial, not a nibble).
- Working tree contains the changes, uncommitted, on the current branch (no `git add`/`commit`/`push`).
</acceptance_criteria>

<output_format>
```
Artifact: <path or "inline">
Canonical patterns:
- <repo>:<path> for <aspect>
- <repo>:<path> for <aspect>

Shipped:
- <behavior>: <files> — verified by `<command>` -> <result>
- <behavior>: <files> — verified by `<command>` -> <result>
- <behavior>: <files> — verified by `<command>` -> <result>

Diff size: <N files, ~M lines net>
Checks: `<command>` -> <pass summary>

Deferred (if any):
- <artifact item>: <one-line reason>

Next: review-branch, then git-commit-push.
```
Keep under 35 lines. No preamble, no sign-off.
</output_format>
