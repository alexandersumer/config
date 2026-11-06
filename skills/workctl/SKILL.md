---
name: workctl
description: Use workctl for credential inspection and renewal, workspace inventory, Git maintenance, and operation recovery.
---

Run the installed `workctl` directly for these tasks. It handles discovery,
renewal, concurrency, recovery, and verification. Use subcommand `--help` only
when an option is unfamiliar.

## Commands

Replace `PROFILE` and `PATH` with the requested credential set and repository or
workspace. Prefer `--json` for structured results.

| Task | Command |
| --- | --- |
| Inspect credential requirements offline | `workctl auth inspect --profile PROFILE --json` |
| Check credentials | `workctl auth preflight --profile PROFILE --check --json` |
| Prepare credentials for work | `workctl auth preflight --profile PROFILE --reuse-valid --json` |
| Explicitly renew credentials | `workctl auth preflight --profile PROFILE --json` |
| Prepare auth and launch a command | `workctl auth exec --profile PROFILE --reuse-valid -- COMMAND` |
| Inspect reset targets | `workctl git reset --list PATH --json` |
| Reset checkouts | `workctl git reset PATH --json` |
| Inventory worktrees | `workctl git worktree inventory PATH --json` |
| Preview seven-day bulk discard | `workctl git worktree clean --older-than 7d --discard-all-local PATH --json` |
| Inspect operation history | `workctl history RUN_ID --unsuccessful --json` |
| Retry unsuccessful resets | `workctl git reset --retry RUN_ID --json` |
| Reconcile uncertain cleanup | `workctl git worktree clean --reconcile RUN_ID --json` |
| Preview worktree cleanup | `workctl git worktree clean PATH --json` |
| Remove eligible worktrees | `workctl git worktree clean --apply PATH --json` |
| Check runtime prerequisites | `workctl doctor --json` |

## Choose and execute

Profiles: `code` = Bitbucket CLI + TWG OAuth + TWG Bitbucket token; `logs` = production Splunk;
`risk` = Risk Mitigation (MFA); `checks` = Pollinator (MFA); `config` = MLP Config;
`metrics` = Phobos; `services` = risk + checks + config + metrics;
`work` = code + logs + services. `bitbucket`, `twg`, and `twg-bitbucket` select
one credential; `runtime` checks the default-cache Kerberos TGT and principal.
Add repeatable `--include-profile runtime` when the selected task needs Kerberos. All built-in SLAuth targets use production.
Choose the narrowest matching set. Use `--config FILE --profile NAME` for exact
account or audience requirements. `auth exec` preserves child output and exit
status; omit `--json` there.

TWG's Bitbucket token is checked by `code`/`work`, never automatically rotated.
When missing or invalid, run `twg setup bitbucket --force` in a terminal; secret
entry stays in that terminal. Use a custom `kerberos` descriptor with `principal`
for an exact identity. Runtime signing keys, service/user-context tokens, browser
sessions, registry auth, and endpoint permissions need provider-specific checks.
Successful default preflight does not prove these. Preflight runs only when
invoked; `auth exec` does not renew credentials during the child command.

Use the requested paths. Preview cleanup, then apply when removal is authorized.
Cleanup removes ignored files in eligible worktrees and retains committed history
in recovery refs. Additional discard flags require authorization for that file
loss. Use ordinary Git workflows for merge or rebase requests.

Read the exit status and JSON outcome. Cleanup's `protected` results are expected
skips, not failures. Report the result and actionable blockers briefly. Workctl
already bounds retries; address a reported failure before repeating the command.

## Cleanup details

- Age selection uses directory creation time; unknown ages are never selected.
- `--exclude PATH` protects a path, its descendants, and its containing worktree
  or repository. `--scope NAME` replaces positional paths with configured roots.
- `--discard-all-local` requires explicit file-loss authorization.
  `--discard-operations` additionally discards abandoned merge/rebase state;
  process locks remain blockers. `--remove-orphans` requires
  `--discard-all-local` and reports unavailable commit recovery.
- Preview before `--apply`. Reconciliation never repeats deletion without a new
  apply and fresh discard decisions.
- `--storage` adds bounded allocation estimates; APFS reclamation is not exact.

Use `doctor --profile PROFILE --managed` for client-executable prerequisites and
managed-install drift, without credential calls or repairs. A client
executable's presence is not verified access.

Read the [guide](references/workctl.md) only for details such as custom profiles,
recovery commands, or strict cleanup policy.
