---
name: forge-spec
description: Build out a spec or plan end-to-end as substantial production-grade work, with canonical patterns and robust verification
argument-hint: "[optional: spec path, plan name, or inline spec]"
inputs:
  - name: spec
    label: Spec or plan
    description: Path to a spec/plan file (e.g. `.plan/foo.md`, `.projects/bar.md`), a plan name to resolve under those directories, or the inline spec text. Leave empty to discover the most recent plan in `.plan` or `.projects`.
    type: string
    required: false
---

<intent>
Deliver the spec below as substantial, working software — not a sketch, not a docs pass, not a stub. Production code, real tests, real wiring, full verification, all green.
</intent>

<spec>
$ARGUMENTS
</spec>

<scope_floor>
This prompt exists because agents tend to nibble. Do not nibble.

- Default expectation: ship the entire spec in one pass. If the spec is genuinely larger than one session, ship a coherent vertical slice that is independently useful and explicitly name what was deferred.
- A "vertical slice" means: real production code, wired into the real entry points, exercised by real tests, callable by a real user or caller. Not a scaffold. Not an interface with no implementation. Not docs describing what would happen.
- Disqualified as substantial work: editing only docs/READMEs/comments, adding only types or interfaces, adding only TODOs, adding only one trivial helper, renaming files, or reformatting. If your diff looks like that, you are not done — return to <implement>.
- Size signal you are aiming at: typically 5+ files touched, hundreds of lines net, multiple new tests with non-trivial assertions, at least one end-to-end behavior demonstrated by a command or test you ran. Use judgement; do not pad.
</scope_floor>

<resolve_spec>
If `<spec>` is empty, discover it:
1. `git ls-files '.plan/*' '.projects/*' '*/PLAN.md' '*/SPEC.md' '*/ROADMAP.md'`.
2. Pick the most recently modified file. Print `Spec: <path>`.
3. If none exist, stop and ask the user for a spec.

If `<spec>` is a path, read it. If it is a bare name (e.g. `auth-rotation`), look it up under `.plan/` and `.projects/` (with and without `.md`). Otherwise treat it as inline spec text.

Read the spec end to end before any other action. If it is ambiguous on scope, intent, or acceptance, ask one batched clarifying question and stop. Do not guess.
</resolve_spec>

<learn_canonical_patterns>
Before writing code, learn the patterns. Run these reads in one parallel batch:
- The target repo's `README.md`, `CONTRIBUTING.md`, and `CLAUDE.md` / `AGENTS.md` if present.
- The directories the spec touches, plus their nearest existing tests.
- Two or three sibling features in the same repo that solve a similar shape of problem, end to end.
- One or two reference repos under `~/atlassian` whose layout and language match the target. Grep for the same problem shape (CLI subcommand, HTTP handler, retry policy, migration, worker, cache, etc.). Extract: naming, layering, error handling, test framework, fixture style, dependency injection, comment density, logging, telemetry, feature-flagging.

Output: `Canonical patterns: <repo>:<path> for <aspect>` lines, one per pattern you will mirror.
</learn_canonical_patterns>

<plan>
Decompose the spec into an ordered task list using the update_todo tool. Each task is one observable behavior with a checkable acceptance signal (a test, a CLI output, an endpoint response, a log line, a measurable metric). Mark tasks complete only after their acceptance signal has been demonstrated, not when the file is saved.

Re-read the spec at the end of <plan>. Every requirement must appear in the task list or in an explicit deferred-slice list with a stated reason.
</plan>

<implement>
Work through every task. Do not stop at the first plausible stopping point.

For each task:
- Build production code, tests, wiring, docs, and config in one pass. Wire new code into the real entry points (CLI, server, worker, public API) — code that exists but is not reachable does not count.
- Mirror the canonical patterns identified above. Reuse existing helpers, types, fixtures, and abstractions. Do not introduce new dependencies, frameworks, or layers unless the spec requires them.
- Tests must fail under at least one named mutation of the new code (off-by-one, swapped args, null vs empty, flipped conditional, wrong exception type, dropped side effect). Note the mutation in the test name or a comment. Cover the happy path, at least one failure path, and at least one edge case per behavior.
- Run the full local check suite after each task. Fix the application code the checker points at. Disallowed: `@Suppress`, lint baselines, dependency bumps, build-config edits, test-infra edits, skipped or `xit`'d tests, `// TODO: implement` placeholders in shipped paths.

Parallelize independent reads and edits in single tool batches. Do not narrate intermediate steps; just do the work.
</implement>

<verify>
Final verification gate, all must pass before you write the report:
- Full local build and test suite green end to end. Capture the exact command and a one-line pass summary.
- Every acceptance signal demonstrated by an actual run (test name + result, CLI invocation + output, HTTP call + response). Not "this should work" — show it worked.
- Spec re-read top to bottom; every requirement maps to a shipped file or test, or appears in `Deferred:` with a one-line reason.
- Diff stays inside the spec's scope; incidental refactors only when required for the change to compile or pass tests.
- The diff clears the <scope_floor>. If it does not, return to <implement>.
</verify>

<acceptance_criteria>
- Spec source line emitted before any file was written.
- Canonical-patterns lines emitted before any file was written, citing concrete repo paths.
- Every spec requirement maps to shipped code or a `Deferred:` entry with reason; nothing dropped silently.
- New code is reachable from real entry points, not orphaned.
- Tests cover happy path, failure path, and edge case per behavior; each new test would fail under a named mutation.
- Full local check suite passed end to end on the final attempt with no disallowed suppressions.
- Diff clears the <scope_floor> (substantial, not a nibble).
- Working tree contains the changes, uncommitted, on the current branch (no `git add`/`commit`/`push`).
</acceptance_criteria>

<output_format>
```
Spec: <path or "inline">
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
- <spec item>: <one-line reason>

Next: review-branch, then git-commit-push.
```
Keep under 35 lines. No preamble, no sign-off.
</output_format>
