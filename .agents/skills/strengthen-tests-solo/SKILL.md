---
name: strengthen-tests-solo
description: Direct test review and edit workflow without subagents. Use only when the user explicitly asks for direct, inline, single-agent, or no-subagent review of tests or changed behavior.
---

# Strengthen Tests Solo

Review and strengthen tests directly in this session. This is not review-only: after validation, make the justified test-code improvements in the workspace instead of merely recommending them. Do not invoke subagents. Preserve the same quality bar as `strengthen-tests` by identifying changed behavior first, finding the real test seam, and implementing only test improvements that catch named realistic regressions.

1. **Get the effective diff and behavior surface automatically.** Do not require a PR, explicit scope, or committed branch changes. Resolve the remote default branch from `origin/HEAD`, falling back to `origin/main` or `origin/master` only if needed; if no remote default exists, omit only the committed-branch part. Build one effective diff from the union of: committed branch changes with `git diff $(git merge-base <remote-default> HEAD)..HEAD`, staged changes with `git diff --cached`, unstaged changes with `git diff`, and untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs. Always include staged, unstaged, and untracked changes even when the committed branch diff exists. If `focus_area` or `$ARGUMENTS` is provided, use it only to narrow this discovered diff. If the effective diff is empty, generated-only, formatter-only, version-bump-only, or has no behavior/test relevance, stop with `no test changes justified` and one short reason.

2. **Read production code first, then discover the test seam.** Identify changed observable behaviors, contracts, public entry points, and failure paths before editing. Read nearby tests if they exist; if they do not, inspect sibling packages/modules, build config, test naming patterns, fixtures, and documented commands until you can name the correct harness and file location for a new test. Treat "no nearby tests" as a reason to add the first focused regression test when there is changed behavior, not as a reason to stop. Find every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` whose directory is an ancestor of any changed production or test file.

3. **Run four direct test-review passes.** Do not report during the passes. Collect candidate improvements only when each is a missing or weak regression signal:
   - **Behavior coverage** — changed public behavior or contract not exercised through a real entry point.
   - **Failure and edge cases** — null/empty/boundary inputs, wrong exception, missing await, stale cache, retries, errors, auth bypass, schema drift.
   - **Assertion strength** — weak assertions, snapshots/golden files that hide the important outcome, tests that would pass with swapped arguments or wrong branches.
   - **Test integration and maintainability** — over-mocking, brittle implementation-detail coupling, missing fixture realism, unclear test names that obscure the regression.

4. **Validate every candidate yourself.** Confirm the changed behavior, realistic regression, existing test gap or absence, public path the test should exercise, and why the proposed location is the minimal maintainable seam. Drop anything that only improves coverage numbers, asserts private implementation details, proves mocks, or cannot catch a named realistic bug.

5. **Implement only validated improvements.** Validated improvements are mandatory edits, not suggestions: modify existing tests or add new test code at the narrowest useful seam. Prefer public behavior over private fields or mock call order. When tests exist, strengthen or extend them at the narrowest useful level. When no suitable test exists, create the smallest idiomatic test file in the discovered harness that exercises the changed public path end-to-end enough to fail for the named regression. Replace weak assertions with exact observable outcomes. Add edge/failure cases only when tied to real changed paths. Skip trivial getters, generated code, framework boilerplate, style conventions, and broad coverage goals.

6. **Run checks.** Run the targeted tests that prove each improvement, then the broader relevant check when available. If no check applies, say why.

7. **Report.** For each touched test, output:

   `<file>::<test_name> — catches <named bug> via <public path> — <command> -> <result>`

   If no validated improvement exists, output `no test changes justified` and one short sentence naming the reviewed behavior surface. If the review surface is too large to validate directly, say `Review limited` and name the residual risk.

Never add tests that only lock in implementation details. Never weaken, skip, delete, or baseline existing checks to get green. Do not use subagents.
