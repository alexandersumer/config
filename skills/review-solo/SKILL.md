---
name: review-solo
description: Direct code-change review without subagents. Use for ordinary, direct, inline, single-agent, or no-subagent review of code/config/test changes, including the design decisions they make. Use review-deep when the user asks for deep, thorough, multi-agent, high-confidence, or heavyweight review.
---

# Review Solo

Review the code directly in this session. Do not invoke subagents. Preserve the same quality bar as `review-deep` by doing explicit, sequential review passes and validating every finding against concrete code, behavior, or conventions.

1. **Get the effective review diff.** No PR, explicit scope, or committed changes are required. Compare against the requested base or PR destination, else a valid `origin/HEAD`, else the sole existing `origin/main` or `origin/master`, and set `<base>` to `git merge-base <comparison-ref> HEAD`. If no unambiguous base exists, say so, use `HEAD`, and review the working-tree portion. Treat `git diff <base>` plus untracked files from `git ls-files --others --exclude-standard` as one final diff; read the committed, staged, and unstaged component diffs only for context, because changes that cancel out are not part of the result. For an explicit staged-only or commit-specific request, use that diff instead. Use `$ARGUMENTS` only to narrow the diff. If the diff is empty, stop with a one-line note. Scale depth to risk; draft status, formatting, or version bumps alone do not make a diff unreviewable. Reuse a prior review only when its scope, code, and context are unchanged and the user has not asked for a fresh one.

2. **Read conventions and real code context.** Find every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` whose directory is an ancestor of any changed file. Read them. Then read enough surrounding code to understand entry points, callers, state/model changes, public contracts, tests, configs, and failure paths touched by the diff. Keep this bounded to the smallest context needed to validate findings.

3. **Run five direct review passes.** Do not report during the passes. Collect candidate issues only when they have a concrete root cause and plausible user/system impact or future change cost:
   - **Correctness** — logic errors, wrong returns, violated contracts, stale state, wrong ordering, broken data flow.
   - **Failure modes** — null/boundary inputs, races, swallowed errors, unclear or misleading error messages, missing diagnostic context, noisy or low-signal logging, leaks, retries, partial failures, regressions in adjacent touched code.
   - **Security** — injection, auth/permission gaps, secrets or sensitive data in logs/errors, unsafe deserialization, missing validation, trust-boundary mistakes.
   - **Conventions** — rules scoped to changed files, local patterns; skip what a linter catches. For changed tests, flag low-value or redundant tests only when they add maintenance cost without a named realistic regression signal or duplicate stronger existing coverage.
   - **Design** — decisions the diff makes that are hard to change later: data models and persisted formats, where contracts live and who controls them, module boundaries, ownership and lifecycle of state, reliance on another component's internals or ordering, compatibility. Collect nothing when the diff makes no such decision.

4. **Validate every candidate yourself.** For each candidate, inspect the relevant file/function/caller/test path in enough detail to confirm or refute it. Prefer concrete execution paths, triggering input, violated rule, or a targeted command. For a design candidate, the evidence is the decision in the diff, what makes it costly to reverse (stored data, external callers, wire formats, another component's internals), and a cheaper option available now. Drop anything you cannot demonstrate concretely. Never keep a finding because it sounds plausible.

5. **Report.** Dedupe by root cause, then rank Critical, High, Medium, Low. For each issue, include severity, `path:line` if known, what is wrong, why it matters, and the minimal fix — one concise paragraph each. If the review surface is too large to validate directly, say `Review limited` and name the unreviewed surface as residual risk. If no validated findings remain, say in one line that no material issue was found and name the reviewed surface.

Never approve, never merge, never invent line numbers. Do not use subagents.
