---
name: review-tests
description: Use when reviewing, strengthening, or adding tests for a pull request, branch, diff, bug fix, feature, or changed behavior.
---

# Review Tests

You do not strengthen tests by guessing from the patch. Fresh-context subagents identify realistic regressions and weak assertions; your job is to provide the changed behavior, relevant production/test code, conventions, and then make only validated test improvements.

1. **Get the diff and behavior surface.** Review `git diff $(git merge-base origin/main HEAD)..HEAD`, narrowed by `focus_area` or `$ARGUMENTS` if provided. If the diff is empty, generated-only, formatter-only, version-bump-only, or has no behavior/test relevance, stop with `no test changes justified` and one short reason.

2. **Read production code first.** Identify the changed observable behaviors, contracts, public entry points, failure paths, and nearby tests before editing. Find every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` whose directory is an ancestor of any changed production or test file and include those conventions in reviewer prompts.

3. **Dispatch four test reviewers in parallel via the Task tool.** Each reviewer gets this prompt verbatim, with `{ROLE}`, `{DIFF}`, `{PRODUCTION_CONTEXT}`, `{TEST_CONTEXT}`, and `{CONVENTIONS}` filled in. No session context — only what you paste:

   > You are reviewing tests as **{ROLE}**. Diff: {DIFF}. Production context: {PRODUCTION_CONTEXT}. Test context: {TEST_CONTEXT}. Conventions: {CONVENTIONS}. Return candidate test improvements with file/test location, the realistic bug each would catch, and why existing tests miss it. One issue = one missing or weak regression signal. Skip coverage theater, style, private implementation details, and mock-call-order assertions unless the public contract is the call.

   Roles:
   - **Behavior coverage** — changed public behavior or contract not exercised through a real entry point
   - **Failure and edge cases** — null/empty/boundary inputs, wrong exception, missing await, stale cache, retries, errors, auth bypass, schema drift
   - **Assertion strength** — weak assertions, snapshots/golden files that hide the important outcome, tests that would pass with swapped arguments or wrong branches
   - **Test integration and maintainability** — over-mocking, brittle implementation-detail coupling, missing fixture realism, unclear test names that obscure the regression

4. **Validate every candidate.** Use one fresh Task subagent per candidate test improvement. Each validator gets this prompt verbatim, with `{ISSUE}`, `{FILES}`, and `{CONVENTIONS}` filled in:

   > Issue: {ISSUE}. Relevant production and test files in full or focused excerpts: {FILES}. Conventions: {CONVENTIONS}. Confirm or refute. State concrete evidence: changed behavior, realistic regression, existing test gap, and the public path the new or changed test should exercise. Score 0–100; anything that does not catch a named realistic bug scores under 80.

   Drop every candidate below 80.

5. **Implement only validated improvements.** Prefer public behavior over private fields or mock call order. Replace weak assertions with exact observable outcomes. Add edge/failure cases only when tied to real changed paths. Skip trivial getters, generated code, framework boilerplate, style conventions, and broad coverage goals.

6. **Run checks.** Run the targeted tests that prove each improvement, then the broader relevant check when available. If no check applies, say why.

7. **Report.** For each touched test, output:

   `<file>::<test_name> — catches <named bug> via <public path> — <command> -> <result>`

   If no validated improvement exists, output `no test changes justified` and one short sentence naming the reviewed behavior surface.

Never add tests that only lock in implementation details. Never weaken, skip, delete, or baseline existing checks to get green.
