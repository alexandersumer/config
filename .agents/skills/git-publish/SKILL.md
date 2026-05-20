---
name: git-publish
description: Commit and publish changes
register_cmd: true
---

Commit current staged, unstaged, and relevant untracked changes, then push to `origin`. Create a PR only when starting from the default branch, using the prescribed CLI path. If the user did not explicitly request publishing, ask before any git write.

Do not publish with a provider-generated title or branch-name subject.

Workflow:
1. Inspect current branch, remote default branch, status, and the effective publish diff for one subject: staged changes, unstaged changes, and relevant untracked files from `git ls-files --others --exclude-standard` rendered as new-file diffs.
2. If no changes, stop.
3. Stage intended changes unless explicitly excluded.
4. If already on a non-default branch: commit and push current branch only. Do not inspect or update PRs unless asked.
5. If on `main`, `master`, or remote default: create a new branch, commit, push, then create a Bitbucket PR using the CLI path below.

Bitbucket PR creation:
- Use CLI only. Do not use Bitbucket MCP to create, list, inspect, or update pull requests during publish.
- Primary path:
  1. Check once for an existing open PR:
     `twg bb prs query --source <branch> --dest <default-branch> -n 5`
  2. If no PR exists, create one:
     `twg bb prs create --title "<subject>" --source <branch> --dest <default-branch>`
     Add `--description "<body>"` only when a grounded PR body is available.
- Do not pass `--reviewer`. Create PRs with no reviewers unless the user explicitly requested reviewers.
- If `twg` is unavailable, use `bb pr create` only when driving the interactive flow and selecting “Skip (no reviewers)”; do not use fully specified non-interactive `bb pr create` because it may apply default reviewers.
- If PR creation fails: check once for an existing branch PR with CLI, retry once with the no-reviewer `twg bb prs create` path, then stop and report the exact blocker.

Do not run tests/builds unless explicitly asked.

Subject regex:
`^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$`

No branch names, issue-title summaries, issue-key prefixes, chat summaries, provider defaults, or trailing period.

Final: commit hash, subject, branch, push/PR result.
