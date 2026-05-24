---
name: review-delegated-tests
description: Default test review. Use unless the user explicitly asks for direct, inline, single-agent, or no-subagent review. Uses fresh-context subagents to review, strengthen, or add tests for changed behavior.
register_cmd: true
---

# Review Tests

You do not strengthen tests by guessing from the patch or by only polishing tests that already exist. Fresh-context subagents identify realistic regressions, missing coverage, and weak assertions; your job is to provide the changed behavior, relevant production code, discovered test harnesses, conventions, and then make only validated test improvements.

1. **Get the effective diff and behavior surface automatically.** Do not require a PR, explicit scope, or committed branch changes. Resolve the remote default branch from `origin/HEAD`, falling back to `origin/main` or `origin/master` only if needed; if no remote default exists, omit only the committed-branch part. Build one effective diff from the union of: committed branch changes with `git diff $(git merge-base <remote-default> HEAD)..HEAD`, staged changes with `git diff --cached`, unstaged changes with `git diff`, and untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs. Always include staged, unstaged, and untracked changes even when the committed branch diff exists. If `focus_area` or `$ARGUMENTS` is provided, use it only to narrow this discovered diff. If the effective diff is empty, generated-only, formatter-only, version-bump-only, or has no behavior/test relevance, stop with `no test changes justified` and one short reason.

2. **Read production code first, then discover the test seam.** Identify the changed observable behaviors, contracts, public entry points, and failure paths before editing. Read nearby tests if they exist; if they do not, inspect sibling packages/modules, build config, test naming patterns, fixtures, and documented commands until you can name the correct harness and file location for a new test. Treat "no nearby tests" as a reason to add the first focused regression test when there is changed behavior, not as a reason to stop. Find every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` whose directory is an ancestor of any changed production or test file and include those conventions in reviewer prompts.

3. **Dispatch four fresh-context test reviewers in parallel.** Before dispatch, confirm `{DIFF}` contains non-empty pasted diff text or focused excerpts, not a path, filename, or summary. Each reviewer gets this prompt verbatim, with `{ROLE}`, `{DIFF}`, `{PRODUCTION_CONTEXT}`, `{TEST_CONTEXT}`, `{TEST_HARNESS}`, and `{CONVENTIONS}` filled in. No session context — only what you paste:

   > You are reviewing tests as **{ROLE}**. Diff: {DIFF}. Production context: {PRODUCTION_CONTEXT}. Existing test context, if any: {TEST_CONTEXT}. Test harness and likely new-test locations: {TEST_HARNESS}. Conventions: {CONVENTIONS}. One issue = one missing or weak regression signal. If there are no tests, propose the smallest high-signal public-behavior test instead of saying there are no improvements. Skip coverage theater, style, private implementation details, and mock-call-order assertions unless the public contract is the call. Return exactly one of:
   >
   > CANDIDATES:
   > - severity: <Critical | High | Medium | Low>
   >   path: <test file path or new test file location>
   >   line: <line or unknown>
   >   claim: <missing or weak regression signal>
   >   evidence: <realistic bug, public path, and why existing tests miss it>
   >   suggested_fix: <minimal test improvement>
   >
   > NO_FINDINGS
   > Reviewed: <files/scope>
   > Reason: <one sentence>

   Roles:
   - **Behavior coverage** — changed public behavior or contract not exercised through a real entry point
   - **Failure and edge cases** — null/empty/boundary inputs, wrong exception, missing await, stale cache, retries, errors, auth bypass, schema drift
   - **Assertion strength** — weak assertions, snapshots/golden files that hide the important outcome, tests that would pass with swapped arguments or wrong branches
   - **Test integration and maintainability** — over-mocking, brittle implementation-detail coupling, missing fixture realism, unclear test names that obscure the regression

4. **Validate reviewer output before candidate validation.** A reviewer response is valid only if it contains either `CANDIDATES:` or `NO_FINDINGS`. Empty, whitespace-only, truncated, or otherwise unstructured output is invalid. If any reviewer output is invalid, retry that reviewer once with a smaller pasted diff/context packet. If it is still invalid, stop with `Review inconclusive`. Never treat invalid output as no findings.

5. **Validate every candidate.** Use one fresh Task subagent per candidate test improvement. Each validator gets this prompt verbatim, with `{ISSUE}`, `{FILES}`, `{TEST_HARNESS}`, and `{CONVENTIONS}` filled in:

   > Issue: {ISSUE}. Relevant production and test files in full or focused excerpts: {FILES}. Test harness and candidate file location: {TEST_HARNESS}. Conventions: {CONVENTIONS}. Confirm or refute. Return `VALIDATED:` with concrete evidence: changed behavior, realistic regression, existing test gap or absence, public path the new or changed test should exercise, why this is the minimal maintainable test location, and score 0–100. Anything that does not catch a named realistic bug scores under 80.

   A validator response is invalid if it is empty, whitespace-only, truncated, or missing `VALIDATED:` with a score and evidence. Retry invalid validator output once with smaller focused production/test excerpts. If it is still invalid, stop with `Review inconclusive`. Drop every candidate below 80.

6. **Implement only validated improvements.** Prefer public behavior over private fields or mock call order. When tests exist, strengthen or extend them at the narrowest useful level. When no suitable test exists, create the smallest idiomatic test file in the discovered harness that exercises the changed public path end-to-end enough to fail for the named regression. Replace weak assertions with exact observable outcomes. Add edge/failure cases only when tied to real changed paths. Reject tests that only prove mocks, test-only production APIs, or implementation details. If a mock becomes more complex than the behavior, prefer a public seam or integration path. Skip trivial getters, generated code, framework boilerplate, style conventions, and broad coverage goals.

7. **Run checks.** Run the targeted tests that prove each improvement, then the broader relevant check when available. If no check applies, say why.

8. **Report.** If any reviewer or validator returned invalid output after retry, output `Review inconclusive` and the failed role. For each touched test, output:

   `<file>::<test_name> — catches <named bug> via <public path> — <command> -> <result>`

   If no validated improvement exists, output `no test changes justified` and one short sentence naming the reviewed behavior surface.

Never add tests that only lock in implementation details. Never weaken, skip, delete, or baseline existing checks to get green.
