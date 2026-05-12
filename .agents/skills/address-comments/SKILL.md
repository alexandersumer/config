---
name: address-comments
description: Address actionable PR comments
---

Address actionable comments on `pr_target`, or the current branch PR. Use `focus` only to narrow scope.

Do not act on every comment. First classify actionable-looking comments:
- Correct: real defect, missing requirement, broken contract, repo convention violation, or meaningful test gap.
- Unclear: requested change is ambiguous.
- Subjective: preference without rule or concrete risk.
- Wrong: contradicted by code, tests, requirements, or accepted branch intent.
- YAGNI: adds ceremony, options, abstraction, or polish not needed for this change.

Fix Correct comments. Ask one clarifying question for Unclear comments. Ignore Subjective, Wrong, and YAGNI comments unless the user explicitly chooses them.

Fetch open review comments. Read the effective PR/worktree diff and relevant file context: PR or base diff, `git diff --cached`, `git diff`, and untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs. If the PR or committed branch diff is empty but the working tree has staged, unstaged, or untracked changes, include those changes instead of treating the review surface as empty. Make the smallest code/test change for each addressed comment. Do not post PR replies.

Run targeted checks. Run the build if available. If no check applies, say why.

Final: list every comment exactly once:
- `<file>:<line> [addressed]`
- `<file>:<line> [ignored: <reason>]`
- `Checks: <command> -> <result>` or `Checks: not run — <reason>`
