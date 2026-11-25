# Global guidance

These rules apply in every repository and to every agent. Repository instructions add to them.

## Environment

- Never send terminal-response escape sequences to a PTY. If detection hangs, restart without a TTY using `TERM=dumb NO_COLOR=1`.
- Keep Slack read-only.
- For signed-in browser tasks, use only my Chrome Work profile through the host's browser extension (the ChatGPT extension in ChatGPT and Codex). Never fall back to another browser or profile.
- Open PRs ready for review, never as drafts: drafts skip automated review.

## Worktrees and evidence

- Use one worktree per task and reuse it for follow-ups. Never edit, stash, reset, or switch branches in canonical checkouts unless asked.
- Create worktrees under a workspace's `.worktrees/` directory, named `<repo>-<task>`. Use the workspace that holds the canonical checkout, or the session's workspace when that one is not writable. Never create them under a temporary directory.
- Keep evidence that a ticket, spec, or PR cites in the repository or in the workspace's `.artifacts/` directory. Never leave it under a temporary directory.

## Tracking

- Work is untracked by default. Create a Jira ticket or a spec only when I start a project, hand one off, or ask. Use the `project` skill for each.
- A session that starts from a ticket key uses that key: name the worktree and branch after it, and update the ticket when the work is blocked or done. Never infer a key for other work.
- If Jira is unreachable, continue the work and say so.

## Writing

- Use American English unless requested otherwise.
- Keep chat replies and drafted messages short and plain: the point first, a few lines, no tables or bullet piles unless asked.
- Put briefs, research summaries, and prep notes in chat, not in new files.
- Updates to an existing page match its length, structure, and tone: if its entries are one line, yours is one line. Change only what is new or wrong.
- Pages often reach leadership: lead with the outcome and say each point once. Cut background, hedging, disclaimers, "X, not Y" framing, and notes on what didn't change.

## Planning and code

- Plans state the end state first, then list every remaining PR with a one-line scope, through rollout and cleanup. No bare counts; mark uncertain items. If new evidence changes the list, say so at once.
- Get the hard-to-change decisions right up front (data models, contracts, boundaries, ownership), designed for where the system is heading. Everything else gets only what the current goal needs.
- Test a design against the code before adopting it. Being raised in a meeting or written in an existing plan is not a reason to keep it.
- Keep PRs small and single-purpose. Prefer removing code to adding it, and flag any PR that adds complexity. New scope becomes a named follow-up.
- Match the surrounding code's structure, naming, and comment density. Tests check behavior, not implementation details.
