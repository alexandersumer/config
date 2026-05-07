---
name: address-comments
description: Address actionable PR comments
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

Address actionable comments on `pr_target`, or the current branch PR. Use `focus` only to narrow scope.

Do not act on every comment. Fix real bugs, edge cases, security issues, architecture problems, misleading names, missing tests, and factual mistakes. Ignore subjective style, unjustified complexity, already-satisfied comments, and comments that contradict branch intent.

Fetch open review comments. Read `git diff base...HEAD` and relevant file context. Make the smallest code/test change for each addressed comment. Do not post PR replies.

Run targeted checks. Run the build if available. If no check applies, say why.

Final: list every comment exactly once:
- `<file>:<line> [addressed]`
- `<file>:<line> [ignored: <reason>]`
- `Checks: <command> -> <result>` or `Checks: not run — <reason>`
