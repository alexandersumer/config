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
`workctl doctor` checks runtime prerequisites without repairs. Use `--profile code` to inspect required client executables without provider calls,
and `--managed` (or `--home PATH`) to compare managed binaries and completions
with the running executable. Service credentials still require `auth preflight --check`;
`config-tools check-install` checks the broader configuration installation.

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
| `workctl auth inspect --profile code` | Resolve credential requirements without provider calls |
| `workctl auth exec -- COMMAND` | Run preflight, then launch a command |
| `workctl git reset [PATH ...]` | Reset primary checkouts to a verified remote branch |
| `workctl git worktree clean [PATH ...]` | Preview linked-worktree removal; `--apply` removes eligible worktrees |
| `workctl git worktree inventory [PATH ...]` | Inspect worktrees, age evidence, local work, operations, and orphans |
| `workctl history [RUN_ID] --unsuccessful` | List runs or inspect unsuccessful targets |
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
| `work` (default) | `code` + `logs` + `services` (eight targets) |
| `code` | Bitbucket CLI OAuth, TWG OAuth, and TWG Bitbucket token |
| `bitbucket`, `twg`, `twg-bitbucket` | One selected client credential |
| `runtime` | Default-cache Kerberos TGT and principal |
| `logs` | Production Splunk |
| `risk` | Risk Mitigation SLAuth, with MFA |
| `checks` | Pollinator SLAuth, with MFA |
| `config` | MLP Config SLAuth |
| `metrics` | Phobos SLAuth for reliability/Tome reads |
| `services` | `risk` + `checks` + `config` + `metrics` |

All built-in SLAuth targets use `prod`. Splunk uses `splunk.paas-inf.net`, group
`atlassian-all`, with MFA; the other audiences require no groups. Select the
narrowest profile or use `--config FILE --profile NAME` for explicit descriptors.
TWG's separate Bitbucket token is checked by `code`/`work`; a missing or invalid
token blocks with `twg setup bitbucket --force`. It is never automatically rotated.
Add `--include-profile runtime` when the task needs Kerberos; use custom descriptors
for an exact principal or service audience. Runtime/service identities, browser
sessions, and registry credentials require their own checks; see the
[coverage boundaries](authentication.md#coverage-evidence-and-boundaries).

### Renewal modes

| Mode | Behavior |
| --- | --- |
| Default | Request one renewal per renewable target, check manual tokens, then verify access; failed renewal remains a failure even if an older credential works |
| `--reuse-valid` | Reuse credentials that meet the validity threshold; renew missing or short-lived credentials |
| `--check` | Check access without explicit refresh or login calls; providers may still renew transparently |

Bitbucket, TWG, Atlas, and Kerberos checks can overlap. Targets within each provider stay
sequential, and browser/MFA issuance runs one flow at a time. Reports retain
profile order. Concurrent preflights share a whole-operation lock; use
`--coordination-timeout SECONDS` to change its wait independently of `--timeout`.

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
Only the selected branch is fetched; unrelated tracking refs and tags are left
alone. The verified fetched commit is used if the branch moves during discovery.

Dirty tracked files, unsafe index state, unfinished Git operations, and local-file
collisions block reset. Replaced branch tips are backed up under
`refs/home-reset-backups/`; detached-HEAD commits receive no additional backup.
Other local branches and linked worktrees are preserved. A batch has no automatic
rollback: earlier changes may remain when a later checkout fails.
Workspace discovery reports and skips orphaned linked worktrees with missing
registrations when their common repository can still be verified. Their files
remain untouched, and other targets continue. Explicit orphan targets and other
invalid metadata still stop discovery before any mutations. Cleanup uses the
same discovery policy; neither operation repairs or deletes orphaned directories.
Recovery commands select the affected checkout explicitly and quote paths and
references for bash and zsh, so they can be run from any directory.

Git maintenance bypasses filesystem monitors for inspection and execution, so a
stale monitor cannot hide local edits from safety checks. This command-specific
override leaves repository configuration unchanged. Checks may take longer on
large working trees.

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
Each eligible preview row identifies local recovery or verified remote publication,
counts local files, and states which file deletions are authorized.

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

## Workspace maintenance

Preview seven-day cleanup before explicitly applying it:

```sh
workctl git worktree clean --older-than 7d --discard-all-local /path/to/work --json
workctl git worktree clean --older-than 7d --discard-all-local --apply /path/to/work --json
```

Age means filesystem **directory creation time**, with a single cutoff per invocation.
Only timestamps strictly before the cutoff qualify. Unknown creation times are reported
and excluded from age selection; commit dates and modification times are never used.
Durations accept positive integer `s`, `m`, `h`, `d`, or `w` units. This is creation age,
not inactivity. Apply discovers the selection again; preview grants no continuing authority.

`--discard-all-local` authorizes tracked, untracked, and ignored file loss only for
selected linked worktrees. `--exclude PATH` excludes the path and descendants before
inspection or mutation. Exclusions must resolve to existing paths. The primary checkout
remains protected. Existing `--discard-local PATH` retains its exact-path meaning.

For reusable roots, create `~/.config/workctl/workspaces.json` (or select an explicit
file with `--workspace-config`):

```json
{"schema_version":1,"scopes":{"work":{"roots":["/path/to/work"],"exclusions":["/path/to/work/keep"]}}}
```

Configured paths are absolute; the file contains paths and policy only. Use
`--scope work` instead of positional paths on cleanup, inventory, or reset.
Additional command-line exclusions are additive, and overlapping roots never duplicate
operations. JSON `selection` reports effective roots and exclusions with path bytes.

`git worktree inventory` reports registered worktrees and protected primary checkouts,
creation evidence, HEAD/branch, local data, operation blockers, and orphan directories.
Use `--storage` for a bounded allocation scan. Allocated bytes, estimated reclaimable
bytes, and observed filesystem free-space changes are separate fields. Scans do not
follow symlinks, deduplicate hardlinks, and report errors as incomplete evidence.
APFS clones, snapshots, and concurrent filesystem activity prevent exact reclamation
promises. Inventory and previews never write committed-history recovery refs.

### Abandoned operations and orphan directories

`--discard-operations` independently authorizes abandoned merge/rebase state in selected
worktrees. It does not authorize local-file loss or bypass process locks. Workctl pins
HEAD and verified merge/rebase commits, including autostash and rewritten commits,
before native Git removal and rechecks
state after recovery. Other operation types and incomplete rebase metadata remain blocked.
In strict mode, operation discard requires exact `--preserve-commits PATH` approval.
Recovery refs preserve commits, including recorded autostashes; they do not capture
additional uncommitted files.

`--remove-orphans --discard-all-local` separately allows directories whose gitfile points
to a missing registration in a verified common repository. Preview first. Uncertain,
symlinked, relative/ambiguous, malformed, nested-repository, and submodule layouts stay
blocked. Orphan removal explicitly reports that commit recovery is unavailable. It
quarantines and revalidates the directory before removal. An uncertain quarantine is
retained for explicit inspection and recorded in history; reconciliation refuses to
silently delete it. Successful removal verifies filesystem and registration absence.

### Operation history and focused recovery

Applied cleanup and nonempty reset runs save intent before mutation, then atomically
publish per-target outcomes in private schema-versioned records under
`~/.local/state/workctl`. `WORKCTL_STATE_DIR` isolates tests or an alternate private
state directory. A state directory must be owned and private (`0700`); records use
`0600`. An intent-publication failure prevents mutation. A later write failure can
leave an incomplete record after mutation; it does not imply rollback. No automatic
history or recovery-ref pruning occurs. Records omit provider output, credentials,
wrapped-command arguments, and Git transcripts.

```sh
workctl history --json
workctl history RUN_ID --unsuccessful --json
workctl git reset --retry RUN_ID --json
workctl git worktree clean --reconcile RUN_ID --json
workctl git worktree clean --reconcile RUN_ID --apply --json
```

Reset retry selects only unsuccessful targets, verifies directory/repository identity,
and performs fresh discovery and normal safety checks. It preserves the original remote
and branch; unsupported option encodings require explicit selection rather than guessing.
Cleanup reconciliation verifies filesystem and registration state first, refuses reused
paths or retained quarantines, and previews remaining work. Deleting remaining work
requires a **new** `--apply` and fresh discard options. Pending, interrupted, malformed,
and incompletely published runs remain distinguishable from success.

### Offline readiness

`auth inspect --profile code --json` resolves required descriptors without calling providers
or reading provider credential stores. Preflight reports client readiness separately while
retaining strict profile success: healthy native `bb` remains visible when TWG's manual
Bitbucket token fails. `auth exec` still requires every declared credential and never
substitutes clients. Doctor checks executable/build provenance, optional profile client
prerequisites, and optional managed installation drift; it never installs, logs in, collects
tokens, or repairs files. Executable presence does not prove version compatibility or
resource authorization; credential checks remain explicit.
