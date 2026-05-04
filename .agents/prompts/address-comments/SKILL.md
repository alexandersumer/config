---
name: address-comments
description: Address actionable pull request review comments with minimal code changes; skip subjective feedback
argument-hint: "[pull request number or URL] [optional focus or instructions]"
inputs:
  - name: pr_target
    label: Pull request number or URL
    description: Pull request number or URL to address comments on. Leave empty to use the current branch's pull request.
    type: string
    required: false
  - name: focus
    label: Focus or instructions
    description: Optional focus area or extra instructions for which comments to address.
    type: string
    required: false
---

Address actionable review comments on `pr_target`, or on the current branch's pull request if none is supplied. Use `focus` only to narrow or clarify; it must not override the classification rules.

Do not satisfy this by appeasing every reviewer sentence. The known failure mode is changing code for subjective preference, adding complexity, or replying instead of fixing. The deliverable is a small code diff that resolves real defects and leaves non-actionable comments alone.

Fetch all open review comments from the repository host. Read the cumulative branch diff with `git diff base...HEAD` and the relevant file context before editing.

Classify every comment:
- Address: correctness bugs, missing edge cases, security concerns, architectural problems, misleading names that can cause misuse, missing tests for changed behavior, factual mistakes.
- Ignore: subjective style, rewrites without correctness/clarity benefit, requests that add unjustified complexity, comments already satisfied, comments that contradict branch intent.

For each addressed comment:
- Make the smallest code/test change that resolves it.
- Keep the change scoped to that comment.
- Do not post PR replies or explanations.

Verification:
- Run targeted checks for changed behavior.
- Run the build if a build command is available.

Final report: every comment exactly once, using only these forms:
- `<file>:<line> [addressed]`
- `<file>:<line> [ignored: <one-line reason>]`

Also include `Checks: <command> -> <result>`.
