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

Target PR: the value supplied as input, or the current branch's PR if none was supplied.

Fetch all open review comments on the target PR using whichever code-review CLI is available (`gh` for GitHub, equivalent for the host in use). Read the cumulative branch diff using three-dot syntax (`git diff base...HEAD`) for full context.

Classify every comment, then act:
- **Address**: bugs, correctness issues, missing edge cases, security concerns, architectural problems, unclear naming, missing tests, factual mistakes. Legitimate regardless of tone.
- **Ignore**: subjective style preferences, rewrites that don't improve correctness or clarity, requests to add complexity, comments already satisfied by existing code, anything contradicting the branch's intent.

For each addressed comment, read the relevant file and surrounding context, then apply the smallest fix that resolves it. Keep the change scoped to the comment; leave unrelated code alone. After all fixes, run the build if a build command is available.

Make code changes only. Do not post replies or explanations to the PR.

Acceptance criteria:
- Every comment appears in the final report exactly once.
- Each entry uses the form `<file>:<line> [addressed]` or `<file>:<line> [ignored: <one-line reason>]`.
- Build passes if a build command exists.
