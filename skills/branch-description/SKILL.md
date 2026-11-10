---
name: branch-description
description: Write or refresh a PR title and description in the canonical tight format and check it. Use whenever a pull request title or description is created, updated, or drafted, including inside git-publish, address-comments, clear-merge-blockers, or an ad hoc PR command, and for Conventional Commit branch summaries.
---

Write the canonical PR title and description for the current branch. Every workflow that creates or updates a PR title or description uses this output, checked by this skill's script. Other skills and repository instructions add title tags or footers; they do not loosen the format.

This skill is read-only. Do not stage, commit, push, or create/update a PR. Use read-only provider commands when they establish the actual PR base or clarify the requested description. Callers publish the checked output unchanged.

## Reader

Write for a reviewer who has not seen the work, and for whoever reads the history later. In under a minute they should know what now happens, why, what proves it, and what to check. The description is not a work log, status report, or investigation record. How the problem was found, commands run, dead ends, test counts, and session notes stay in chat or linked documents.

When rules conflict, follow this order: accuracy, then the limits, then completeness. Drop detail rather than exceed a limit. If the change cannot be explained within the limits, or it bundles unrelated changes, say it should be split instead of writing a longer description.

## Effective change set

Do not use the branch name, issue title, provider default, conversation recency, your own last action, the latest commit, or a vague summary.

Use the requested base or target PR destination as `<comparison-ref>`. Otherwise use `origin/HEAD` when it resolves to an existing remote-tracking branch, or else the sole existing `origin/main` or `origin/master`. Set `<base>` with `git merge-base <comparison-ref> HEAD`. If no unambiguous ref or merge base exists, report that gap and inspect the working tree.

Build one effective change set from every available source: `git log <base>..HEAD --oneline` and `git diff <base>..HEAD` when `<base>` exists, staged changes with `git diff --cached`, unstaged changes with `git diff`, and untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs. Always include staged, unstaged, and untracked changes even when the committed branch diff exists. If the committed branch diff is empty but the working tree has changes, describe those changes instead of treating the branch as empty. Ignore generated files, lockfiles, and formatting noise unless they are the change.

When the effective change set is empty, output exactly this diagnostic sentence and stop, replacing placeholders:

```text
No branch description to write: current branch `<current-branch>` has no changes relative to comparison ref `<comparison-ref>`, and there are no staged, unstaged, or untracked changes.
```

Use `git branch --show-current`, or `HEAD` if detached, and the actual comparison ref. If no ref or merge base exists, use this diagnostic:

```text
No branch description to write: current branch `<current-branch>` has no resolved comparison base, and there are no staged, unstaged, or untracked changes.
```

The cumulative effective diff is the source of truth. Use conversation context only to clarify intent the diff supports. Identify the highest-level user-visible or system-behavior change that explains most meaningful diffs; read changed code, callers, or tests when the diff is implementation-heavy. Logging, tests, cleanup, refactors, and follow-up fixes appear only when they support the main change, unless they are its purpose.

## Title

The first line is the PR title and a valid Conventional Commit subject:

```text
^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$
```

Use a lowercase imperative description of the branch-level change, at most 72 characters after the `: `, with no trailing period, emoji, or issue key. Use `!` only with a `BREAKING CHANGE:` footer. When repository instructions require title tags, such as a severity label, put them first as `[tag]` tokens followed by a space; nothing else goes before the subject.

## Body

Use this shape. Only the opening paragraph is required; include each labeled section only when it adds something, in this order, at most once.

```text
<Opening: one or two sentences stating what now happens, in behavior terms.>

**Why:** <The problem or need, and who it affects.>

**Changes:**
- <One behavior-level change per bullet.>

**Evidence:** <The strongest observed proof.>

**Rollout:** <What turns it on and how to undo it.>

**Review:** <One question that needs the reviewer's judgment.>

**Follow-up:** <Deferred work, in one sentence.>

<Footers, such as Refs: ABC-123 or BREAKING CHANGE: ...>
```

- **Opening:** at most 50 words. Lead with the outcome a reviewer can verify, not the implementation. For small changes it is the whole body.
- **Why:** include it unless the opening already makes the reason obvious. Describe what is wrong without the change, in present tense, and why this approach fixes it. Summarize linked discussions instead of relying on the link.
- **Changes:** only for two or more notable changes. Two to six flat bullets of one sentence, 25 words or fewer each, described as behavior. No file inventories; name a file or identifier only when the reviewer must find it.
- **Evidence:** only proof you observed: a measured before/after, a reproduction, or the test that fails without the change. One or two sentences, optionally followed by one small table of before/after numbers. Never list commands run or test counts, and say CI passed only when you saw it pass.
- **Rollout:** only for gated, risky, migrating, or hard-to-reverse changes. If a risky change has no control, write "No flag; revert to roll back."
- **Review:** only when a decision needs the reviewer, named specifically, such as "Is 30s the right timeout for slow uploads?"
- **Follow-up:** only for deferred work a reviewer would otherwise ask about.
- **Footers:** Conventional Commit trailers after a blank line: `Refs:`, `Fixes:`, `BREAKING CHANGE:`. Host attribution lines are allowed after them.

## Hard limits

- Body of 200 words or fewer, excluding footers and attribution; aim for 80 to 150. Raise the limit with `--max-words` only when the user asks for a longer description.
- Opening of 50 words or fewer; Changes lists of six bullets or fewer, each 25 words or fewer.
- Every paragraph after the opening starts with one of the labels above, written exactly as shown, with a colon. A list or table sits directly under its label; a table only under Evidence.
- No headings, horizontal rules, HTML, nested lists, or bold outside the labels.

## Write it plainly

- Describe behavior ("close the connection when the client disconnects"), not abstractions ("improve lifecycle handling"). Prefer concrete numbers to adjectives.
- Explain internal names by what they do, or drop them.
- State facts directly. Avoid "This PR," first person, "ensures," "robust," "comprehensive," hedging, and em dashes.
- Never invent motivation, results, risk, flag states, or test status.

## Check

Write the title and body to files and run:

```bash
python3 <skill-directory>/scripts/check_pr_description.py --title "<title>" --body-file <body-file>
```

Fix every reported problem and rerun until it prints `ok`. Never return or publish a description that fails the check. The check covers structure and length only. Then reread as the reviewer: the opening alone should say what changes, each section should say something new, and no claim should go beyond the evidence.

## Updating an existing PR

When a PR's diff changes after review fixes, a target merge that changes scope, or new commits, regenerate the title and body from the current cumulative diff and replace both. Never append "Update:" notes or changelogs; review history belongs in PR comments.

## Output

Return the title line, a blank line, and the body, with nothing else. Do not output commit hashes, branch names, push or PR results, or labels like `Subject:`. The empty-change diagnostics above are the only exception.

## Examples

These are fictional. Never reuse their facts or names.

```markdown
fix(exports): free the export slot when a client disconnects

Abandoned exports now release their slot as soon as the client disconnects, so new exports stop queuing behind them.

**Why:** A slot stays held until its export finishes, even after the client leaves. A few abandoned 2 GB exports block every other user for up to an hour.

**Evidence:** `export-slots.test.ts` disconnects mid-export and expects the slot back within 1s; it fails without the change.

Refs: EXP-311
```

```markdown
[medium-risk] perf(search): cache tenant permission lookups for one minute

Search results now load in under 300ms at p95 for large tenants, down from 1.8s.

**Why:** Every result row checks permissions with its own database call, so a 200-row page makes 200 calls.

**Changes:**
- Cache each user's permission set per tenant for 60 seconds.
- Drop a user's cached set when their group membership changes.

**Evidence:** Staging load test, 200-row pages, 50 concurrent users:

| Metric | Before | After |
| --- | --- | --- |
| p95 latency | 1.8s | 0.29s |
| Database calls per page | 200 | 1 |

**Rollout:** Behind `search-permission-cache`; turning it off restores per-row checks.

**Review:** Is 60 seconds of stale permissions acceptable after a manual revoke?
```
