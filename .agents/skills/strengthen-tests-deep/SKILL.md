---
name: strengthen-tests-deep
description: Heavyweight test review and edit workflow using fresh-context Reviewer candidate generation, direct Axiom validation, edits, and checks. Use when the user asks for deep, thorough, multi-agent, high-confidence, or heavyweight test strengthening or regression coverage improvements. Use strengthen-tests-solo for ordinary/direct single-agent test strengthening.
---

# Strengthen Tests Deep

This is the heavyweight test-strengthening path. You do not strengthen tests by guessing from the patch or by only polishing tests that already exist. This is not review-only: after validation, make the justified test-code improvements in the workspace instead of merely recommending them. Fresh-context Reviewers identify candidate realistic regressions, missing coverage, and weak assertions. Use Axiom's configured Reviewer callable agent via direct `invoke_agent` only; do not simulate Reviewers with `claude`, `codex`, `cursor-agent`, `acpx`, `toad`, or any other external agent CLI. Reviewer output is candidate evidence, not authority. Axiom owns validation, editing, checks, and final judgment.

1. **Get the effective diff and behavior surface automatically.** Do not require a PR, explicit scope, or committed branch changes. Resolve the remote default branch from `origin/HEAD`, falling back to `origin/main` or `origin/master` only if needed; if no remote default exists, omit only the committed-branch part. Build one effective diff from the union of: committed branch changes with `git diff $(git merge-base <remote-default> HEAD)..HEAD`, staged changes with `git diff --cached`, unstaged changes with `git diff`, and untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs. Always include staged, unstaged, and untracked changes even when the committed branch diff exists. If `focus_area` or `$ARGUMENTS` is provided, use it only to narrow this discovered diff. If the effective diff is empty, generated-only, formatter-only, version-bump-only, or has no behavior/test relevance, stop with `no test changes justified` and one short reason.

2. **Read production code first, then discover the test seam.** Identify the changed observable behaviors, contracts, public entry points, and failure paths before editing. Read nearby tests if they exist; if they do not, inspect sibling packages/modules, build config, test naming patterns, fixtures, and documented commands until you can name the correct harness and file location for a new test. Treat "no nearby tests" as a reason to add the first focused regression test when there is changed behavior, not as a reason to stop. Find every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` whose directory is an ancestor of any changed production or test file and include those conventions in reviewer prompts.

3. **Invoke four fresh-context test Reviewer passes directly.** Before invocation, confirm `{DIFF}` contains non-empty pasted diff text or focused excerpts, not a path, filename, or summary. Use Axiom's configured Reviewer callable agent via direct, foreground `invoke_agent` for each role. Run roles sequentially unless the runtime provides direct non-wrapper parallel `invoke_agent`; never use scheduled/background/wrapper delegation or external agent CLIs as a substitute. Each Reviewer pass gets this prompt verbatim, with `{ROLE}`, `{DIFF}`, `{PRODUCTION_CONTEXT}`, `{TEST_CONTEXT}`, `{TEST_HARNESS}`, and `{CONVENTIONS}` filled in. No session context — only what you paste:

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

4. **Validate Reviewer output shape before candidate validation.** A Reviewer response is valid only if it contains either `CANDIDATES:` or `NO_FINDINGS`. Empty, whitespace-only, truncated, unavailable, timed out, or otherwise unstructured output is invalid. If any Reviewer output is invalid, retry that Reviewer once with a smaller pasted diff/context packet through direct `invoke_agent`. If it is still invalid, stop with `Review inconclusive`. Never treat invalid output as no findings, and never fall back to `claude`, `codex`, `cursor-agent`, `acpx`, `toad`, or another agent CLI.

5. **Validate every candidate directly in Axiom.** Do not delegate validation of Reviewer findings to another subagent, validator, or recursive review chain. For each candidate test improvement, Axiom must inspect the relevant production and test files, changed behavior, public entry point, existing test gap, test harness, and conventions itself. Confirm or refute with concrete evidence: named realistic regression, public path the new or changed test should exercise, why existing tests miss it, why the proposed file is the minimal maintainable test seam, and why the assertion would fail for the bug. Drop anything Axiom cannot demonstrate concretely with score-equivalent confidence of at least 80. Anything that does not catch a named realistic bug is unvalidated. Unvalidated candidates are never implemented or reported as justified improvements; if required evidence cannot be obtained after focused inspection, mark the review inconclusive instead of forwarding the candidate.

6. **Implement only directly validated improvements.** Validated improvements are mandatory edits, not suggestions: modify existing tests or add new test code at the narrowest useful seam. Prefer public behavior over private fields or mock call order. When tests exist, strengthen or extend them at the narrowest useful level. When no suitable test exists, create the smallest idiomatic test file in the discovered harness that exercises the changed public path end-to-end enough to fail for the named regression. Replace weak assertions with exact observable outcomes. Add edge/failure cases only when tied to real changed paths. Reject tests that only prove mocks, test-only production APIs, or implementation details. If a mock becomes more complex than the behavior, prefer a public seam or integration path. Skip trivial getters, generated code, framework boilerplate, style conventions, and broad coverage goals.

7. **Run checks.** Run the targeted tests that prove each improvement, then the broader relevant check when available. If no check applies, say why.

8. **Report.** If any Reviewer returned invalid output after retry or Axiom cannot obtain required validation evidence, output `Review inconclusive` and the failed role or evidence gap. For each touched test, output:

   `<file>::<test_name> — catches <named bug> via <public path> — <command> -> <result>`

   If no validated improvement exists, output `no test changes justified` and one short sentence naming the reviewed behavior surface.

Never add tests that only lock in implementation details. Never weaken, skip, delete, or baseline existing checks to get green.
