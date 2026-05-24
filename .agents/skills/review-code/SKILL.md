---
name: review-code
description: Default code-change review using fresh-context subagents. Use for PRs, diffs, branches, staged changes, unstaged changes, untracked files, or any set of code/config/test changes unless the user explicitly asks for direct, inline, single-agent, or no-subagent review.
register_cmd: true
---

# Review Code

You do not review the code directly. Subagents do the reviewing in fresh context because session history biases the reviewer toward the author's framing and burns tokens on noise. Your job is dispatching and aggregating.

1. **Get the effective review diff automatically.** Do not require a PR, explicit scope, or committed branch changes. Resolve the remote default branch from `origin/HEAD`, falling back to `origin/main` or `origin/master` only if needed; if no remote default exists, omit only the committed-branch part. Build one effective diff from the union of: committed branch changes with `git diff $(git merge-base <remote-default> HEAD)..HEAD`, staged changes with `git diff --cached`, unstaged changes with `git diff`, and untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs. Always include staged, unstaged, and untracked changes even when the committed branch diff exists. If `focus_area` or `$ARGUMENTS` is provided, use it only to narrow this discovered diff. If the effective review diff is empty, draft-only, formatter-only, version-bump-only, or already reviewed in this thread, stop with a one-line note.

2. **Read conventions yourself.** Find every `CLAUDE.md`, `AGENTS.md`, or `REVIEW.md` whose directory is an ancestor of any changed file. Read them and paste the relevant convention text into reviewer prompts.

3. **Dispatch four fresh-context reviewers in parallel.** Before dispatch, confirm `{DIFF}` contains non-empty pasted diff text or focused excerpts, not a path, filename, or summary. Each reviewer gets this prompt verbatim, with `{ROLE}`, `{DIFF}`, and `{CONVENTIONS}` filled in. No session context — only what you paste:

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
   - **Failure modes** — null/boundary inputs, races, swallowed errors, leaks, regressions in adjacent code the diff touches
   - **Security** — injection, auth, secrets, unsafe deserialization, missing validation
   - **Conventions** — rules scoped to changed files; skip what a linter catches

4. **Validate reviewer output before candidate validation.** A reviewer response is valid only if it contains either `CANDIDATES:` or `NO_FINDINGS`. Empty, whitespace-only, truncated, or otherwise unstructured output is invalid. If any reviewer output is invalid, retry that reviewer once with a smaller pasted diff/context packet. If it is still invalid, stop with `Review inconclusive`. Never treat invalid output as no findings.

5. **Validate every candidate.** Use one fresh Task subagent per candidate issue. Each validator gets this prompt verbatim, with `{ISSUE}`, `{FILES}`, and `{CONVENTIONS}` filled in:

   > Issue: {ISSUE}. Relevant files in full: {FILES}. Conventions: {CONVENTIONS}. Confirm or refute. Return `VALIDATED:` with concrete evidence — triggering input, line that executes wrong, rule violated — and score 0–100. Anything you cannot demonstrate concretely scores under 80.

   A validator response is invalid if it is empty, whitespace-only, truncated, or missing `VALIDATED:` with a score and evidence. Retry invalid validator output once with smaller focused file excerpts. If it is still invalid, stop with `Review inconclusive`. Drop every candidate below 80.

6. **Report.** Dedupe by root cause, then rank Critical, High, Medium, Low. For each issue, include severity, `path:line`, what is wrong, why it matters, and the fix — one sentence each. End with **Ready to merge** only if every reviewer returned valid output and no validated findings remain, **Review inconclusive** if any reviewer or validator returned invalid output after retry, or **Needs attention**/**Needs work** if validated findings remain. If zero candidates survive validation, say so in one line.

Never approve, never merge, never invent line numbers. Subagents see only what you paste.
