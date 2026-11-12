---
name: strengthen-tests
description: Strengthen regression coverage by reviewing production behavior, validating test gaps, editing tests, and proving realistic bugs are caught. Supports diff-derived changes and explicitly scoped existing behavior.
---

# Strengthen Tests

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Review and strengthen tests directly in this session. This is not review-only: make the validated test-code improvements in the workspace. Valid improvements may add, strengthen, consolidate, or delete tests, and each must improve regression signal for a named realistic bug.

1. **Get the effective diff and behavior surface.** No PR, explicit scope, or committed changes are required. Compare against the requested base or PR destination, else a valid `origin/HEAD`, else the sole existing `origin/main` or `origin/master`, and set `<base>` to `git merge-base <comparison-ref> HEAD`. If no unambiguous base exists, say so, use `HEAD`, and review the working-tree portion. Treat `git diff <base>` plus untracked files from `git ls-files --others --exclude-standard` as one final diff; component diffs are context only, because changes that cancel out are not part of the result. For an explicit staged-only or commit-specific request, use that diff instead. If `$ARGUMENTS` names a behavior, contract, test, file, module, or subsystem, use it as the scope even without a matching diff. Call the target either diff-derived behavior or explicitly scoped existing behavior. Otherwise, if the diff is empty, generated-only, formatter-only, version-bump-only, or has no behavior or test relevance, stop with `no test changes justified` and one short reason.

2. **Read production code first, then find the test seam.** Name the target observable behaviors, contracts, public entry points, and failure paths before editing. If no nearby tests exist, find the correct harness and file location from sibling modules, build config, and fixtures, and add the first focused regression test; absence of tests is not a reason to stop. Read every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` in an ancestor directory of any scoped file.

3. **Run four test-review passes**, collecting candidates without reporting yet:
   - **Behavior coverage** — target public behavior or contract not exercised through a real entry point.
   - **Failure and edge cases** — null/empty/boundary inputs, wrong exception, unclear user/operator-facing error message, missing await, stale cache, retries, errors, auth bypass, schema drift.
   - **Assertion strength** — weak assertions, snapshots/golden files that hide the important outcome, tests that would pass with swapped arguments or wrong branches.
   - **Integration and maintainability** — over-mocking, implementation-detail coupling, unrealistic fixtures, names that obscure the regression, lower-level tests dominated by stronger public-path coverage.

4. **Validate every candidate yourself.** For additions or retained touched tests, confirm the realistic regression, the existing gap, the public path to exercise, and why the location is the minimal maintainable seam. For deletions, confirm the test is non-contract noise or name the retained test that catches the same bug through an equal or stronger path. Punctuation-only, docs-text-only, snapshot-only, mock-only, private-detail, and trivial-branch tests are low value unless the exact output or interaction is a stable public/operator contract. Drop anything that only raises coverage numbers, proves mocks, duplicates stronger coverage, or cannot catch a named realistic bug.

5. **Implement every validated improvement** at the narrowest useful seam. Prefer exact observable outcomes over private fields or mock call order. When no suitable test exists, create the smallest idiomatic test file in the discovered harness that fails for the named regression. Assert error messages or log records only when they are a public/operator contract or catch a realistic regression; avoid brittle wording and log-volume assertions. Skip trivial getters, generated code, framework boilerplate, and broad coverage goals.

6. **Run checks** under the proof policy: targeted tests for each improvement, broader checks only when justified. For each new or materially strengthened regression test, run or reuse safe fail-then-pass proof under `prove-check`; if that proof is unsafe or impractical, say it was not run and do not claim the test catches the bug. If no check applies, say why.

7. **Report.** For each touched test, output one of:

   `<file>::<test_name> — catches <named bug> via <public path> — <fail-then-pass proof>`
   `<file>::<test_name> — covers <behavior> via <public path>; regression-catching proof not run — <reason> — <command> -> <result>`
   `<file>::<test_name> — removed as <low-value|redundant> because <reason>; retained coverage <test or contract> — <command> -> <result>`

   If no validated improvement exists, output `no test changes justified` and one short sentence naming the reviewed behavior surface. If the review surface is too large to validate directly, say `Review limited` and name the residual risk.

Never add or retain touched tests that only lock in implementation details. Never weaken, skip, delete, or baseline existing checks to get green.

## Optional independent test review

Only when the user explicitly requests a broad coverage audit or independent reviewers: use the harness's native managed subagents as focused fresh-context reviewers, with roles and count chosen by risk. Pass only the explicit behavior scope, relevant production and test excerpts, harness details, and scoped conventions; never inherited session history or prior conclusions. Reviewer output is candidates; validate each directly before editing. This agent owns edits, checks, and final judgment. For an explicitly single-agent request, keep all work in this session. If an independent pass is unavailable or incomplete, report that limitation and continue direct work without claiming the audit is complete.
