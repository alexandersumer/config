---
name: strengthen-tests-deep
description: Heavyweight test review and edit workflow using fresh-context Reviewer candidate generation, direct validation, edits, and checks. Use only when the user asks for deep, thorough, multi-agent, high-confidence, or heavyweight test strengthening. Use strengthen-tests-solo for ordinary or direct test and regression-coverage improvements.
---

# Strengthen Tests Deep

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

This is the heavyweight test-strengthening path. You do not strengthen tests by guessing from the patch or by only polishing tests that already exist. This is not review-only: after validation, make the justified test-code improvements in the workspace instead of merely recommending them. Fresh-context Reviewers identify candidate realistic regressions, missing coverage, weak assertions, and low-value or redundant tests. Use the current harness's native managed subagent mechanism. If no native managed mechanism can provide separate fresh contexts and collect a terminal result from each, stop with `Review inconclusive` and name the missing capability. Do not simulate Reviewers with external agent CLIs or unmanaged processes. Reviewer output is candidate evidence, not authority. This agent owns validation, editing, checks, and final judgment.

1. **Get the effective diff and behavior surface automatically.** Do not require a PR, explicit scope, or committed branch changes. Resolve the remote default branch from `origin/HEAD`, falling back to `origin/main` or `origin/master` only if needed; if no remote default exists, omit only the committed-branch part. Build one effective diff from the union of: committed branch changes with `git diff $(git merge-base <remote-default> HEAD)..HEAD`, staged changes with `git diff --cached`, unstaged changes with `git diff`, and untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs. Always include staged, unstaged, and untracked changes even when the committed branch diff exists. If `focus_area` or `$ARGUMENTS` names a behavior, contract, test, file, module, or subsystem, use it as an explicit test-strengthening scope without requiring a matching diff. Call the resulting target behavior either diff-derived behavior or explicitly scoped existing behavior. Otherwise, if the effective diff is empty, generated-only, formatter-only, version-bump-only, or has no behavior/test relevance, stop with `no test changes justified` and one short reason.

2. **Read production code first, then discover the test seam.** Identify the target observable behaviors, contracts, public entry points, and failure paths before editing. Read nearby tests if they exist; if they do not, inspect sibling packages/modules, build config, test naming patterns, fixtures, and documented commands until you can name the correct harness and file location for a new test. Treat "no nearby tests" as a reason to add the first focused regression test when target behavior exists, not as a reason to stop. Find every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` whose directory is an ancestor of any scoped production or test file and include those conventions in reviewer prompts.

3. **Invoke four fresh-context test Reviewer passes through native managed subagents.** Before invocation, confirm `{DIFF_OR_SCOPE}` contains non-empty pasted diff text or a precise explicit behavior scope, with focused production excerpts in `{PRODUCTION_CONTEXT}`. Create one separately prompted context per role. Give each Reviewer only its role prompt and the explicitly constructed evidence packet; do not pass inherited session context, prior Reviewer output, or conclusions from this session. Run roles concurrently when the harness safely supports it. Otherwise run them sequentially or in bounded waves as separate fresh contexts; limited concurrency changes latency, not the review contract. Collect every initial result before validating output or candidates. Do not use external agent CLIs, unmanaged wrappers or processes, detached or scheduled execution, or recursive validation delegation. Each Reviewer pass gets this prompt verbatim, with `{ROLE}`, `{DIFF_OR_SCOPE}`, `{PRODUCTION_CONTEXT}`, `{TEST_CONTEXT}`, `{TEST_HARNESS}`, and `{CONVENTIONS}` filled in:

   > You are reviewing tests as **{ROLE}**. Diff or explicit behavior scope: {DIFF_OR_SCOPE}. Production context: {PRODUCTION_CONTEXT}. Existing test context, if any: {TEST_CONTEXT}. Test harness and likely new-test locations: {TEST_HARNESS}. Conventions: {CONVENTIONS}. One issue = one missing or weak regression signal, or one low-value/redundant test. If there are no tests, propose the smallest high-signal public-behavior test instead of saying there are no improvements. Skip coverage theater, style, private implementation details, and mock-call-order assertions unless the public contract is the call. Flag punctuation-only, docs-text-only, snapshot-only, mock-only, or duplicate tests only when they do not protect a stable public/operator contract. Return exactly one of:
   >
   > CANDIDATES:
   > - severity: <Critical | High | Medium | Low>
   >   path: <test file path or new test file location>
   >   line: <line or unknown>
   >   claim: <missing/weak signal or low-value/redundant test>
   >   evidence: <realistic bug, public path, and gap, or retained coverage showing redundancy>
   >   suggested_fix: <minimal test improvement, consolidation, or deletion>
   >
   > NO_FINDINGS
   > Reviewed: <files/scope>
   > Reason: <one sentence>

   Roles:
   - **Behavior coverage** — target public behavior or contract not exercised through a real entry point
   - **Failure and edge cases** — null/empty/boundary inputs, wrong exception, unclear user/operator-facing error message, missing await, stale cache, retries, errors, auth bypass, schema drift
   - **Assertion strength** — weak assertions, snapshots/golden files that hide the important outcome, tests that would pass with swapped arguments or wrong branches
   - **Test integration and maintainability** — over-mocking, brittle implementation-detail coupling, missing fixture realism, unclear test names that obscure the regression, redundant lower-level tests dominated by stronger public-path coverage

4. **Validate Reviewer output shape before candidate validation.** A Reviewer response is valid only if it contains either `CANDIDATES:` or `NO_FINDINGS`. Empty, whitespace-only, truncated, unavailable, timed out, or otherwise unstructured output is invalid. After collecting all initial results, retry only invalid roles once through the same native managed mechanism with a smaller evidence packet and a new fresh context. Run multiple retries concurrently when safely supported; otherwise run them separately. Do not rerun valid roles. If any retried role is still invalid, stop with `Review inconclusive`. Never treat invalid output as no findings, and never fall back to external agent CLIs or unmanaged processes.

5. **Validate every candidate directly in this session.** Do not delegate validation of Reviewer findings to another subagent, validator, or recursive review chain. For each candidate test improvement, inspect the relevant production and test files, target behavior, public entry point, existing test gap, test harness, and conventions yourself. Confirm or refute additions with concrete evidence: named realistic regression, public path the new or changed test should exercise, why existing tests miss it, why the proposed file is the minimal maintainable test seam, and why the assertion would fail for the bug. Confirm deletions by showing the test is non-contract noise or naming the retained test that catches the same bug through an equal or stronger path. Drop anything you cannot demonstrate concretely with score-equivalent confidence of at least 80. Anything that does not catch a named realistic bug or remove a proven low-value/redundant test is unvalidated. Unvalidated candidates are never implemented or reported as justified improvements; if required evidence cannot be obtained after focused inspection, mark the review inconclusive instead of forwarding the candidate.

6. **Implement only directly validated improvements.** Validated improvements are mandatory edits, not suggestions: modify, add, consolidate, or delete test code at the narrowest useful seam. Prefer public behavior over private fields or mock call order. When tests exist, strengthen or extend them at the narrowest useful level, or delete redundant/low-value tests after naming the retained coverage. When no suitable test exists, create the smallest idiomatic test file in the discovered harness that exercises the target public path end-to-end enough to fail for the named regression. Replace weak assertions with exact observable outcomes. Assert error messages or log records only when they are part of the public/operator contract or catch a realistic regression; avoid brittle wording checks and log-volume assertions. Add edge/failure cases only when tied to real target paths. Reject tests that only prove mocks, test-only production APIs, or implementation details. If a mock becomes more complex than the behavior, prefer a public seam or integration path. Skip trivial getters, generated code, framework boilerplate, style conventions, and broad coverage goals.

7. **Run checks.** Validate through the proof policy: reuse proof when valid, otherwise run the targeted tests that prove each improvement, then the broader relevant check only when justified. For each new or materially strengthened regression test, run or reuse safe fail-then-pass proof under `prove-check`; if that proof is unsafe or impractical, say it was not run and do not claim the test catches the bug. If no check applies, say why.

8. **Report.** If any Reviewer returned invalid output after retry or you cannot obtain required validation evidence, output `Review inconclusive` and the failed role or evidence gap. For each touched test, output one of:

   `<file>::<test_name> — catches <named bug> via <public path> — <fail-then-pass proof>`
   `<file>::<test_name> — covers <behavior> via <public path>; regression-catching proof not run — <reason> — <command> -> <result>`
   `<file>::<test_name> — removed as <low-value|redundant> because <reason>; retained coverage <test or contract> — <command> -> <result>`

   If no validated improvement exists, output `no test changes justified` and one short sentence naming the reviewed behavior surface.

Never add or retain touched tests that only lock in implementation details. Never weaken, skip, delete, or baseline existing checks to get green.
