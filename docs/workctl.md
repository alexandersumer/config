# workctl

`workctl` is a Rust CLI for work credentials, Git checkouts, and linked worktrees.

[Install](#install) · [Commands](#commands) · [Authentication](#authentication) ·
[Git operations](#git-operations) · [Output](#output) · [Exit status](#exit-status)

## Install

From a checkout of this repository, with Rust and Cargo installed:

```sh
cargo run --locked --release -- install-workctl
workctl doctor
```

Add `~/.local/bin` to `PATH` if needed. For zsh completion, add
`~/.local/share/zsh/site-functions` to `fpath` before `compinit`. The repository's
zsh configuration already does this. Open a fresh shell after installation.

Git operations require Git 2.36 or later. Authentication uses the installed `bb`,
`twg`, and `atlas` provider CLIs; each provider keeps its own credentials.
`workctl doctor` checks runtime prerequisites without repairs. It does not verify
service credentials or managed installation; use `auth preflight --check` and
`config-tools check-install` for those checks.

<details>
<summary>Managed files and installation behavior</summary>

The installer updates these managed files:

| File | Purpose |
| --- | --- |
| `~/.local/bin/workctl` | Public CLI |
| `~/.local/bin/config-tools` | Configuration installer and repository checks |
| `~/.local/share/zsh/site-functions/_workctl` | Generated zsh completions |

Binary replacement is atomic. The installer checks managed-file ownership before
replacing files and creates completion directories with permissions accepted by
zsh. Use `--home PATH` to install into an isolated home.

The retired `~/.local/bin/reset_to_origin` is removed only when its bytes match
the current installer or the previous managed `config-tools` binary. Unknown files
and symlinks block migration and remain untouched. The installer does not rewrite
shell startup files. The repository's zsh configuration removes retired aliases
and functions and refreshes command hashes when loaded.

</details>

## Commands

| Command | Purpose |
| --- | --- |
| `workctl auth preflight` | Renew credentials and verify access |
| `workctl auth exec -- COMMAND` | Run preflight, then launch a command |
| `workctl git reset [PATH ...]` | Reset primary checkouts to a verified remote branch |
| `workctl git worktree clean [PATH ...]` | Preview linked-worktree removal; `--apply` removes eligible worktrees |
| `workctl doctor` | Check Git, executable, temporary storage, process groups, and locks |
| `workctl completions SHELL` | Generate shell completions |

Use `--help` after any command for options and examples. Help and completion
generation do not access repositories. Git paths default to the current directory;
use `--` before paths that begin with a dash.

## Authentication

```sh
workctl auth preflight
workctl auth preflight --profile code --check
workctl auth exec --profile code -- bb pr list
```

### Profiles

| Profile | Credentials |
| --- | --- |
| `work` (default) | Bitbucket, TWG, and production Splunk |
| `code` | Bitbucket and TWG |
| `logs` | Production Splunk |

Splunk uses SLAuth for `splunk.paas-inf.net`, group `atlassian-all`, with MFA.
Use `--config FILE --profile NAME` for an explicit custom profile.

### Renewal modes

| Mode | Behavior |
| --- | --- |
| Default | Request one renewal per target, then verify access; failed renewal remains a failure even if an older credential works |
| `--reuse-valid` | Reuse credentials that meet the validity threshold; renew missing or short-lived credentials |
| `--check` | Check access without explicit refresh or login calls; providers may still renew transparently |

Complete browser or MFA prompts when requested. `--non-interactive` permits
silent refresh but prevents browser/MFA issuance. Known expiry must meet
`--min-valid-for` (default: 300 seconds). Unknown expiry is reported explicitly.
Access is checked at handoff; future access is not guaranteed.

`auth exec` launches its child only after successful preflight. It preserves argv,
stdin, stdout, environment, signals, and child exit status. The preflight report
goes to stderr. It does not monitor or renew credentials during child execution.

[Authentication reference](authentication.md): provider behavior, deadlines,
credential ownership, custom profiles, and failure handling.

## Git operations

### Reset checkouts

Inspect targets before resetting:

```sh
workctl git reset --list ~/src
workctl git reset ~/src
```

Reset fetches and verifies the remote target, then resets the selected primary
checkouts. The default target comes from the remote's advertised HEAD. Use
`--remote NAME --branch NAME` to select a target explicitly.

Dirty tracked files, unsafe index state, unfinished Git operations, and local-file
collisions block reset. Replaced branch tips are backed up under
`refs/home-reset-backups/`; detached-HEAD commits receive no additional backup.
Other local branches and linked worktrees are preserved. A batch has no automatic
rollback: earlier changes may remain when a later checkout fails.

### Clean up linked worktrees

Preview first, then apply:

```sh
workctl git worktree clean ~/src
workctl git worktree clean --apply ~/src
```

Cleanup protects active tracked and untracked work and retains local branches.
Before removal, it saves and verifies committed HEAD in
`refs/workctl/cleanup/<HEAD>`. Default cleanup works offline and never pushes.

**Applying cleanup removes ignored files in eligible worktrees.** Those files can
include local configuration and secrets. Preview shows the affected paths and
counts. Recovery refs preserve commits, not removed ignored files.

Use `--strict` to require fresh remote-publication proof and protect ignored files.
Explicit discard options apply only to the named worktrees. Locks, nested
repositories, populated submodules, missing paths, and uncertain ownership still
block removal. Primary checkouts are always protected.

### Scope and execution

| Supplied path | Reset selects | Cleanup selects |
| --- | --- | --- |
| Workspace directory | Primary checkouts found beneath it | Registered linked worktrees beneath it |
| Primary checkout | That checkout | All its registered linked worktrees, including paths outside it |
| Linked worktree | Refused | That linked worktree only |

Reset defaults to at most 32 workers; cleanup defaults to at most four. Both accept
`--jobs N`, where `N` is 1 through 32. Run Git operations while repositories are otherwise idle. Workctl
locks coordinate its own invocations, not editors or unrelated Git commands.

[Git reference](git.md): exact selection rules, recovery commands, discard
options, revalidation, concurrency, deadlines, and JSON fields.

## Output

Results go to stdout. Progress and diagnostics go to stderr. Git operations show
interactive progress or periodic plain updates when redirected. Auth reports each
provider's status, account or scope, remaining lifetime, and any recovery action.
Status words remain readable without color. `NO_COLOR`, `CLICOLOR=0`, and
`TERM=dumb` disable color; redirected streams contain no terminal decoration.

`--json` works before or after a subcommand and emits one JSON document on stdout.
Results include `schema_version`, `operation`, and `status`. Field details belong
to the relevant command's reference.

| Command | JSON schema |
| --- | --- |
| `auth preflight` | 1 |
| `git reset` | 1 |
| `git worktree clean` | 2 |
| `doctor` | 1 |
| `completions` | 1; generated script is in `script` |
| `auth exec` | Unsupported; child stdout is preserved |

Help, version, and usage errors follow normal CLI conventions even with `--json`.

## Exit status

| Code | Meaning |
| --- | --- |
| `0` | Success, including cleanup that protects active local work |
| `1` | Failed operation, blocked authentication, or cleanup safety blocker |
| `2` | Invalid usage |
| `130` | Interrupted |

After successful preflight, `auth exec` preserves the child's exit status or
termination signal. Launch failures return `127` for a missing command or `126`
for another launch error.

## Reference

- [Authentication](authentication.md)
- [Git operations](git.md)
- [Configuration and installation](configuration.md)
- [Development and verification](development.md)
- [Repository README](../README.md)
