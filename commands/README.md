# gap-commands

Shared coding commands (slash commands / skills) for the Gen AI Platform team.

| Command | Description |
|---|---|
| `git-commit` | Stage all changes and commit |
| `git-commit-push` | Stage, commit, push (auto-branches if protected) |
| `review-branch` | Review diff for bugs, security, architecture |
| `describe-branch` | Generate PR description (3 sentences + summary) |
| `describe-diff` | Summarize working changes in <10 words |
| `apply-changes` | Implement changes with surgical precision |
| `fix-failures` | Run build/test and fix failures properly |
| `sync-main` | Fetch latest main and merge it into the current branch |
| `resolve-conflict` | Resolve merge conflicts preserving branch intent |
| `clean-up-feature-flag` | Remove a fully rolled-out feature flag |
| `regenerate-detekt-baseline` | Regenerate detekt baselines for Main and Test source sets |

## Setup

**Cursor** — for each command, create a skill directory with a symlink:

```sh
mkdir -p ~/.cursor/skills-cursor/git-commit
ln -s /path/to/gap-commands/git-commit.md ~/.cursor/skills-cursor/git-commit/SKILL.md
```

**OpenCode** — symlink the repo into your OpenCode config:

```sh
ln -s /path/to/gap-commands opencode/commands
```

## Adding a command

Create a `.md` file at the repo root:

```
---
name: my-command
description: What it does
---
Instructions for the agent.
```
