# workctl

`workctl` is a Rust CLI for work credentials, Git checkouts, and linked worktrees.

[Install](#install) · [Commands](#commands) · [Authentication](#authentication) ·
[Git operations](#git-operations) · [Workspace maintenance](#workspace-maintenance) · [Output](#output) · [Exit status](#exit-status)

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
| `workctl auth preflight` | Verify credentials and renew those that need it |
| `workctl auth inspect --profile code` | Resolve credential requirements without provider calls |
| `workctl auth exec -- COMMAND` | Run preflight, then launch a command |
| `workctl pull-request print-review-request PULL_REQUEST ...` | Print a review request using the current titles of open Bitbucket Cloud pull requests |
| `workctl pull-request approve --author-team TEAM_ID WORKSPACE/REPOSITORY ...` | Approve open Bitbucket Cloud pull requests assigned to you in the specified repositories |
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
| `work` (default) | `code` + `logs` + `services` + `micros` (nine targets) |
| `code` | Bitbucket CLI OAuth, TWG OAuth, and TWG Bitbucket token |
| `bitbucket`, `twg`, `twg-bitbucket` | One selected client credential |
| `runtime` | Default-cache Kerberos TGT and principal |
| `micros` | Production `micros-server` MFA cache and Micros user login |
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
| `--force` | Explicitly renew supported credentials even when current access is valid |
| `--reuse-valid` | Compatibility flag for default reuse |
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

## Pull requests

### Print a review request

```sh
workctl pull-request print-review-request \
  https://bitbucket.org/workspace/repository/pull-requests/123 \
  'workspace/another-repository#456'
```

Pass one or more open Bitbucket Cloud pull requests as URLs or quoted
`WORKSPACE/REPOSITORY#ID` references. The command prints
`Could someone please review these?` followed by one `#ID: Title` line per pull
request in argument order. Repeated references to the same pull request are
deduplicated. Titles come from Bitbucket, with whitespace collapsed to keep each
title on one line. The command makes no Bitbucket writes and uses no LLM.

The command uses native `bb` OAuth and curl 7.84 or later. Use
`--non-interactive` to allow silent refresh without browser login. If a pull
request is closed, inaccessible, or has no usable title, the command reports
the failed reference and prints no message. With `--json`, the completed message is
in `message`.

### Approve pull requests

```sh
workctl pull-request approve --author-team TEAM_ID --dry-run workspace/repository workspace/another-repository
workctl pull-request approve --author-team TEAM_ID workspace/repository https://bitbucket.org/workspace/another-repository
```

Pass one or more Bitbucket Cloud repositories as `WORKSPACE/REPOSITORY` or as
repository URLs. The command deduplicates arguments and reads every page in those
repositories before posting any approvals. The `pr` alias remains available for
existing invocations. Choose exactly one author scope. Use `--author-email EMAIL` or
`--author-account-id ACCOUNT_ID` for one author, `--author-team TEAM_ID` for
team members, or `--all-authors` to allow any author.

The command uses `curl` for the Bitbucket Cloud API and native `bb` OAuth through
the `bitbucket` preflight. It reuses valid credentials, refreshes them when needed,
and may open a browser if sign-in is required. Use `--non-interactive` to allow
only silent refresh.

The command uses the authenticated user's Bitbucket UUID. It skips existing
approvals and historical approvals visible in Bitbucket's activity log, including
approvals that were reset or withdrawn. Immediately before posting, it checks
that the pull request is open, you are still assigned as a reviewer, and you have
neither authored nor already approved it. Bitbucket does not expose a conditional approval
endpoint, so an external change can still occur between that check and the POST.

`--dry-run` reads the same eligibility evidence without posting approvals and
can run while another invocation holds the approval lock. Live runs share a
local lock across repositories to prevent duplicate approvals. If the lock is
held, the command reports the wait on stderr and continues once it acquires the
lock. Press Ctrl-C to cancel, or use `--lock-timeout SECONDS` to bound the wait.
Use `--lock-timeout 0` to fail immediately if another run holds the lock.
Repository discovery and pull request verification use up to four workers.
Discovery finishes for every repository before any approval. Bitbucket narrows discovery by reviewer
and, when the team filter fits in the query, author. The command checks both
again before approval. Current approvals found during discovery need no further
requests.

On HTTP 429, workers share a cooldown. The command honors `Retry-After` and
backs off when the header is absent. It stops dispatching requests when it
exhausts the retry limit or the 60-second cooldown budget. A rate-limited
approval can retry after fresh eligibility checks. If the write outcome is
uncertain, the command reads back the approval and reports any uncertainty.
Rate-limit handling requires curl 7.84 or later.

If Bitbucket rejects a credential during the run, the command attempts one
silent renewal for the original account and verifies its Bitbucket UUID.
Workers share the renewed credential and recheck eligibility before sending
an approval. If renewal or verification fails, the command stops dispatching
requests and reports the remaining work.

The terminal report summarizes approvals, prior approvals, skipped items, and
failures. A preview lists eligible pull requests, and errors identify the affected
pull request on stderr. Add `--json` for a schema-versioned result for each pull
request checked. Failed or unverified items produce a nonzero exit status. A rerun
uses fresh Bitbucket state.

### Restrict approvals to one author

```sh
workctl pull-request approve --author-email jane.doe --dry-run workspace/repository
workctl pull-request approve --author-email jane.doe@example.com --dry-run workspace/repository
workctl pull-request approve --author-account-id ACCOUNT_ID --dry-run workspace/repository
```

A short value such as `jane.doe` expands to `jane.doe@atlassian.com`. A full
address uses its supplied domain. TWG must return one active account with that
exact email, ignoring case. If the lookup can't verify a
single account, the command stops before contacting Bitbucket. Use the exact
Atlassian account ID to skip directory lookup and TWG authentication. The JSON
report includes the selected ID as `author_account_id` once it is known.

If you only know the person's name, run `twg user search "Jane Doe"` and check
the candidate's email or account ID before proceeding. Names and Bitbucket
nicknames can be shared by multiple accounts. Use `--dry-run` to preview the
matching pull requests, then remove it to approve eligible requests.

### Restrict approvals to an Atlassian team

```sh
workctl pull-request approve --author-team TEAM_ID --dry-run workspace/repository
```

Use the UUID in the Atlassian team profile URL as `TEAM_ID`. Workctl reads the
active team's full roster through `twg` before contacting Bitbucket. It matches
full members to pull request authors by Atlassian account ID and checks the
author again immediately before approval. An empty roster or an author without
a matching account ID yields no approval. Directory errors, incomplete
pagination, and responses for a different team stop the run before any approval.
The team roster is read once per invocation. Add `--json` to include the team ID
in the report.

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
same default discovery policy. Reset never deletes orphans; cleanup requires the
separate [orphan-removal options](#abandoned-operations-and-orphan-directories).
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
Exact-path discard options apply only to named worktrees; bulk discard applies only
to the resolved selection. Locks, nested
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

Reset defaults to at most 32 workers and lowers concurrency after interrupted Git
transfers. Cleanup defaults to at most four workers. Both accept `--jobs N`, where
`N` is 1 through 32. Run Git operations while repositories are otherwise idle. Workctl
locks coordinate its own invocations, not editors or unrelated Git commands.

[Git reference](git.md): exact selection rules, recovery commands, discard
options, revalidation, concurrency, deadlines, and JSON fields.

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
inspection or mutation. Excluding a descendant protects its entire worktree or repository
from removal or reset. Suggested apply commands retain these exclusions and age limits.
Exclusions must resolve to existing paths. The primary checkout
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

Reset publishes each target's start and outcome as it happens. Finished targets
record their final phase, attempt count, recovery refs, and failure category.
Retry runs eligible unfinished targets and performs fresh discovery and normal
safety checks. An active run must finish first. Changed identities and targets
that may have already changed the checkout are reported individually and require
inspection. Retry preserves the original remote, branch, and available execution
limits. Pass `--jobs`, `--attempts`, or `--timeout` to override those limits.
Cleanup records stable blocker categories so later inspection can distinguish
known protections from a different current result.
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

## Output

Results go to stdout. Progress and diagnostics go to stderr. Git operations show
interactive progress or periodic plain updates when redirected. Auth reports each
provider's status, account or scope, remaining lifetime, and any recovery action.
Status words remain readable without color. `NO_COLOR`, `CLICOLOR=0`, and
`TERM=dumb` disable color; redirected streams contain no terminal decoration.

`--json` works before or after a subcommand and emits one JSON document on stdout.
Results include `schema_version` and `operation`; execution results also include
`status`. History listing returns `runs`, while run inspection returns the persisted
operation record. Field details belong
to the relevant command's reference.

| Command | JSON schema |
| --- | --- |
| `auth preflight`, `auth inspect` | 1 |
| `pull-request approve`, `git reset` | 1 |
| `pull-request print-review-request` | 1; review request text in `message` |
| `git worktree clean` | 2 |
| `git worktree inventory` | 1 |
| `history` (listing or stored run) | 1 |
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
