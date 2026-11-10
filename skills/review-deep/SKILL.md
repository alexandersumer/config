---
name: review-deep
description: Heavyweight code-change review using fresh-context Reviewer candidate generation and direct validation by this agent. Use when the user asks for deep, thorough, multi-agent, high-confidence, or heavyweight review of PRs, diffs, branches, staged changes, unstaged changes, untracked files, or any set of code/config/test changes, including the design decisions they make. Use review-solo for ordinary/direct single-agent review.
---

# Review Deep

This is the heavyweight code-review path. Fresh-context Reviewers generate candidate evidence because session history biases the reviewer toward the author's framing and burns tokens on noise. Use the current harness's native managed subagent mechanism. If no native managed mechanism can provide separate fresh contexts and collect a terminal result from each, stop with `Review inconclusive` and name the missing capability. Do not simulate Reviewers with external agent CLIs or unmanaged processes. Reviewer output is candidate evidence, not authority. This agent owns validation and final judgment.

1. **Get the effective review diff.** No PR, explicit scope, or committed changes are required. Compare against the requested base or PR destination, else a valid `origin/HEAD`, else the sole existing `origin/main` or `origin/master`, and set `<base>` to `git merge-base <comparison-ref> HEAD`. If no unambiguous base exists, say so, use `HEAD`, and review the working-tree portion. Treat `git diff <base>` plus untracked files from `git ls-files --others --exclude-standard` as one final diff; read the committed, staged, and unstaged component diffs only for context, because changes that cancel out are not part of the result. For an explicit staged-only or commit-specific request, use that diff instead. Use `$ARGUMENTS` only to narrow the diff. If the diff is empty, stop with a one-line note. Scale depth to risk; draft status, formatting, or version bumps alone do not make a diff unreviewable. Reuse a prior review only when its scope, code, and context are unchanged and the user has not asked for a fresh one.

2. **Read conventions and focused code context yourself.** Find every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` whose directory is an ancestor of any changed file. Read enough surrounding unchanged code to understand affected entry points, callers, state/model transitions, public contracts, tests, configs, and failure paths. Put focused excerpts in `{CODE_CONTEXT}`; when the diff is genuinely self-contained, use a one-sentence reason instead of fabricating context.

3. **Invoke five fresh-context Reviewer passes through native managed subagents.** Before invocation, confirm `{DIFF}` contains non-empty pasted diff text or focused excerpts, not a path, filename, or summary, and `{CODE_CONTEXT}` contains focused surrounding context or the explicit self-contained reason. Create one separately prompted context per role. Give each Reviewer only its role prompt and the explicitly constructed evidence packet; do not pass inherited session context, prior Reviewer output, or conclusions from this session. Run roles concurrently when the harness safely supports it. Otherwise run them sequentially or in bounded waves as separate fresh contexts; limited concurrency changes latency, not the review contract. Collect every initial result before validating output or candidates. Do not use external agent CLIs, unmanaged wrappers or processes, detached or scheduled execution, or recursive validation delegation. Each Reviewer pass gets this prompt verbatim, with `{ROLE}`, `{DIFF}`, `{CODE_CONTEXT}`, and `{CONVENTIONS}` filled in:

   > You are reviewing a code change as **{ROLE}**. Diff: {DIFF}. Relevant unchanged caller, contract, and failure-path context: {CODE_CONTEXT}. Conventions: {CONVENTIONS}. One issue = one root cause. Skip nitpicks, style, "consider also". If it is not a real defect, risk, or costly design decision, drop it. For changed tests, a low-value or redundant test is a valid issue only when it adds maintenance cost without a named realistic regression signal or duplicates stronger existing coverage. Return exactly one of:
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
   - **Design** — decisions the diff makes that are hard to change later: data models and persisted formats, where contracts live and who controls them, module and trust boundaries, ownership and lifecycle of state, reliance on another component's internals or ordering, compatibility; return `NO_FINDINGS` when the diff makes none

4. **Validate Reviewer output shape before candidate validation.** A Reviewer response is valid only if it contains either `CANDIDATES:` or `NO_FINDINGS`. Empty, whitespace-only, truncated, unavailable, timed out, or otherwise unstructured output is invalid. After collecting all initial results, retry only invalid roles once through the same native managed mechanism with a smaller evidence packet and a new fresh context. Run multiple retries concurrently when safely supported; otherwise run them separately. Do not rerun valid roles. If any retried role is still invalid, stop with `Review inconclusive`. Never treat invalid output as no findings, and never fall back to external agent CLIs or unmanaged processes.

5. **Validate every candidate directly in this session.** Do not delegate validation of Reviewer findings to another subagent, validator, or recursive review chain. For each candidate, inspect the relevant files, functions, callers, tests, and conventions yourself. Confirm or refute the issue with concrete evidence: triggering input or state, execution path, the line or rule violated, and realistic user/system impact. For a design candidate, the evidence is the decision in the diff, what makes it costly to reverse (stored data, external callers, wire formats, another component's internals), and a cheaper option available now. Drop anything you cannot demonstrate concretely; never forward an unvalidated candidate as a finding. If a candidate looks material but the evidence needed to settle it is inaccessible, name it as an evidence gap instead.

6. **Report.** Dedupe by root cause, then rank Critical, High, Medium, Low. For each issue, give severity, `path:line`, what is wrong, why it matters, and the minimal fix in one concise paragraph. List evidence gaps after findings. End with **Needs work** when validated findings remain, **Review inconclusive** when any Reviewer output is still invalid after retry, or otherwise **No material findings** in one line naming the reviewed surface.

Never approve, never merge, never invent line numbers. Reviewers see only what you paste through direct Reviewer invocation. This agent validates directly and owns final judgment.
