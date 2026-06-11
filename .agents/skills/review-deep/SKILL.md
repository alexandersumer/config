---
name: review-deep
description: Heavyweight code-change review using fresh-context Reviewer candidate generation and direct validation by this agent. Use when the user asks for deep, thorough, multi-agent, high-confidence, or heavyweight review of PRs, diffs, branches, staged changes, unstaged changes, untracked files, or any set of code/config/test changes. Use review-solo for ordinary/direct single-agent review.
---

# Review Deep

This is the heavyweight code-review path. Fresh-context Reviewers generate candidate evidence because session history biases the reviewer toward the author's framing and burns tokens on noise. Use the current harness's configured direct reviewer-agent mechanism. If the current harness does not expose a direct reviewer-agent mechanism, stop with `Review inconclusive` and name the missing capability. Do not simulate Reviewers by shelling out to arbitrary agent CLIs or unmanaged wrappers. Reviewer output is candidate evidence, not authority. This agent owns validation and final judgment.

1. **Get the effective review diff automatically.** Do not require a PR, explicit scope, or committed branch changes. Resolve the remote default branch from `origin/HEAD`, falling back to `origin/main` or `origin/master` only if needed; if no remote default exists, omit only the committed-branch part. Build one effective diff from the union of: committed branch changes with `git diff $(git merge-base <remote-default> HEAD)..HEAD`, staged changes with `git diff --cached`, unstaged changes with `git diff`, and untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs. Always include staged, unstaged, and untracked changes even when the committed branch diff exists. If `focus_area` or `$ARGUMENTS` is provided, use it only to narrow this discovered diff. If the effective review diff is empty, draft-only, formatter-only, version-bump-only, or already reviewed in this thread, stop with a one-line note.

2. **Read conventions yourself.** Find every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` whose directory is an ancestor of any changed file. Read them and paste the relevant convention text into reviewer prompts.

3. **Invoke four fresh-context Reviewer passes directly.** Before invocation, confirm `{DIFF}` contains non-empty pasted diff text or focused excerpts, not a path, filename, or summary. Use the direct foreground reviewer-agent facility exposed by the current harness for each role. Run roles sequentially unless the harness provides direct parallel reviewer calls; never use scheduled/background/wrapper delegation or arbitrary external agent CLIs as a substitute. Each Reviewer pass gets this prompt verbatim, with `{ROLE}`, `{DIFF}`, and `{CONVENTIONS}` filled in. No session context — only what you paste:

   > You are reviewing a code change as **{ROLE}**. Diff: {DIFF}. Conventions: {CONVENTIONS}. One issue = one root cause. Skip nitpicks, style, "consider also". If it is not a real defect or risk, drop it. Return exactly one of:
   >
   > CANDIDATES:
   > - severity: <Critical | High | Medium | Low>
   >   path: <file path>
   >   line: <line or unknown>
   >   claim: <what is wrong>
   >   evidence: <specific code, behavior, rule, or failure path>
   >   suggested_fix: <minimal fix>
   >
   > NO_FINDINGS
   > Reviewed: <files/scope>
   > Reason: <one sentence>

   Roles:
   - **Correctness** — logic errors, wrong returns, violated contracts
   - **Failure modes** — null/boundary inputs, races, swallowed errors, unclear or misleading error messages, missing diagnostic context, noisy or low-signal logging, leaks, regressions in adjacent code the diff touches
   - **Security** — injection, auth, secrets or sensitive data in logs/errors, unsafe deserialization, missing validation
   - **Conventions** — rules scoped to changed files; skip what a linter catches

4. **Validate Reviewer output shape before candidate validation.** A Reviewer response is valid only if it contains either `CANDIDATES:` or `NO_FINDINGS`. Empty, whitespace-only, truncated, unavailable, timed out, or otherwise unstructured output is invalid. If any Reviewer output is invalid, retry that Reviewer once with a smaller pasted diff/context packet through the same direct reviewer mechanism. If it is still invalid, stop with `Review inconclusive`. Never treat invalid output as no findings, and never fall back to arbitrary external agent CLIs.

5. **Validate every candidate directly in this session.** Do not delegate validation of Reviewer findings to another subagent, validator, or recursive review chain. For each candidate, inspect the relevant files, functions, callers, tests, and conventions yourself. Confirm or refute the issue with concrete evidence: triggering input or state, execution path, the line or rule violated, and realistic user/system impact. Drop anything you cannot demonstrate concretely with score-equivalent confidence of at least 80. Unvalidated candidates are never reported as findings; if required evidence cannot be obtained after focused inspection, mark the review inconclusive instead of forwarding the candidate.

6. **Report.** Dedupe by root cause, then rank Critical, High, Medium, Low. For each issue, include severity, `path:line`, what is wrong, why it matters, and the fix — one sentence each. End with **No material findings** only if every Reviewer returned valid output and no directly validated findings remain, **Review inconclusive** if any Reviewer output is invalid after retry or you cannot obtain required validation evidence, or **Needs attention**/**Needs work** if directly validated findings remain. If zero candidates survive validation, say so in one line.

Never approve, never merge, never invent line numbers. Reviewers see only what you paste through direct Reviewer invocation. This agent validates directly and owns final judgment.
