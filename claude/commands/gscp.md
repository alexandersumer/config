---
description: Git stage, commit, and push with auto-generated message
---

Stage all working tree changes, commit with a clear message, and push to remote.

## Steps

1. Run `git status` (never use `-uall`) and `git diff` to analyze changes
2. If no changes exist (clean working tree), report "Nothing to commit" and stop
3. Stage all changes with `git add -A`
4. Generate a commit message:
   - Single sentence, sentence case
   - Accurately summarize the primary change
   - No trailing punctuation, emojis, or conventional commit prefixes
5. Commit using HEREDOC (EOF must not be indented):
   ```
   git commit -m "$(cat <<'EOF'
   Message here
   EOF
   )"
   ```
6. Push: use `git push` if upstream exists, otherwise `git push -u origin HEAD`
7. Verify with `git status`
