---
name: workctl
description: Use workctl for credential checks and renewal, checkout resets, and linked-worktree cleanup.
---

Run the installed `workctl` directly for these tasks. It handles discovery,
renewal, concurrency, recovery, and verification. Use subcommand `--help` only
when an option is unfamiliar.

## Commands

Replace `PROFILE` and `PATH` with the requested credential set and repository or
workspace. Prefer `--json` for structured results.

| Task | Command |
| --- | --- |
| Check credentials | `workctl auth preflight --profile PROFILE --check --json` |
| Prepare credentials for work | `workctl auth preflight --profile PROFILE --reuse-valid --json` |
| Explicitly renew credentials | `workctl auth preflight --profile PROFILE --json` |
| Prepare auth and launch a command | `workctl auth exec --profile PROFILE --reuse-valid -- COMMAND` |
| Inspect reset targets | `workctl git reset --list PATH --json` |
| Reset checkouts | `workctl git reset PATH --json` |
| Preview worktree cleanup | `workctl git worktree clean PATH --json` |
| Remove eligible worktrees | `workctl git worktree clean --apply PATH --json` |
| Check runtime prerequisites | `workctl doctor --json` |

## Choose and execute

Profiles: `code` = Bitbucket + TWG; `logs` = production Splunk; `work` = both.
Choose the narrowest matching set. Use `--config FILE --profile NAME` for exact
account or audience requirements. `auth exec` preserves child output and exit
status; omit `--json` there.

Use the requested paths. Preview cleanup, then apply when removal is authorized.
Cleanup removes ignored files in eligible worktrees and retains committed history
in recovery refs. Additional discard flags require authorization for that file
loss. Use ordinary Git workflows for merge or rebase requests.

Read the exit status and JSON outcome. Cleanup's `protected` results are expected
skips, not failures. Report the result and actionable blockers briefly. Workctl
already bounds retries; address a reported failure before repeating the command.

Read the [guide](../../../docs/workctl.md) only for details such as custom profiles,
recovery commands, or strict cleanup policy.
