---
description: Git stage and commit with auto-generated message
---

Stage all working tree changes and commit with a clear, descriptive message.

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
6. Verify with `git status`
