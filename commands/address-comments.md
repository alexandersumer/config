---
name: address-comments
description: Look at PR review comments and robustly address legitimate ones, ignore noise
argument-hint: "[PR number or URL] [optional focus or instructions]"
inputs:
  - name: pr_target
    label: PR number or URL
    description: PR number or URL to address comments on. Leave empty to use the current branch's PR.
    type: string
    required: false
---

The user may have provided a PR number, URL, or additional instructions as input — use them if present. Otherwise default to the current branch's PR.

Fetch all open review comments on the target PR using `gh pr view --json reviews,comments` and `gh pr review-comment list` (or equivalent `gh` CLI commands available). Read the branch diff to understand the full context of the changes.

For each comment, assess legitimacy:
- **Address**: bugs, correctness issues, missing edge cases, security concerns, architectural problems, unclear naming, missing tests, factual mistakes. These are legitimate regardless of tone.
- **Ignore**: personal style preferences with no objective basis, subjective rewrites that don't improve correctness or clarity, requests to add unnecessary complexity, comments already addressed by existing code, and anything that contradicts the intent of the branch.

When addressing a legitimate comment, read the relevant file and surrounding context first. Apply the fix with surgical precision — do not refactor beyond what the comment requests. After all fixes are applied, run the build if a build command is available.

Do not leave a comment response or explanation — just make the code changes.

Report a brief summary at the end: list each comment with `[addressed]` or `[ignored: reason]`.
