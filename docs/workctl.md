# Git maintenance with workctl

[Back to README](../README.md)

Install the workstation CLI, reset primary checkouts, and preview or remove idle linked worktrees. For credential checks, see [Authentication readiness](authentication.md).

`workctl` is the public workstation tool. Git maintenance and authentication readiness are its public domains;
configuration installation and CI remain internal `config-tools` commands.
There are no standalone checkout-reset or worktree-cleanup aliases.

```sh
workctl git reset ~/atlassian ~/oss ~/src ~/stable
workctl git worktree clean ~/atlassian ~/oss ~/src ~/stable
workctl git worktree clean --apply ~/atlassian ~/oss ~/src ~/stable
workctl auth preflight
workctl auth exec --profile code -- bb pr list
workctl doctor
workctl completions zsh
```

## Install workctl

Build with `cargo build --locked --release`. Install only the workstation CLI with
`cargo run --release -- install-workctl`; use `--home PATH` for an isolated home.
The full configuration installer also installs it. New completion directories use permissions accepted by zsh’s security checks,
including when the shell has a group-writable umask. Both update the internal managed
`config-tools` installer and install the compiled binary
atomically at `~/.local/bin/workctl` and generated zsh completion at
`~/.local/share/zsh/site-functions/_workctl`. The tracked zsh configuration adds
that directory to `fpath` before Oh My Zsh initializes completions. For another
zsh configuration, add it to `fpath` before `compinit` yourself.

The migration removes `~/.local/bin/reset_to_origin` only when it is an executable
regular file byte-identical to the current installer or the previously installed
managed `config-tools` binary. Unknown files and symlinks block migration and
remain untouched. No shell startup files are rewritten by `install-workctl`.
Use a fresh shell or reload your shell configuration after cutover; the tracked zsh
configuration removes the retired function and alias and refreshes command hashes.

## Command conventions

Every public command has Clap-generated help, version information, and examples.
Use `--` before paths beginning with a dash. Git options belong to their operation:
`--jobs` applies to reset and cleanup, and `--apply` belongs to cleanup. No paths means `.`;
there are no hardcoded workspace roots. Help and completions require no Git access.
Exit codes are 0 for success, 1 for operational failure or blocked cleanup,
2 for invalid usage, and 130 for interruption.

## Reset checkouts

```sh
workctl git reset --list .
workctl git reset --jobs 4 --attempts 3 --timeout 300 /path/to/workspace
workctl git reset --remote upstream --branch release /path/to/repository
workctl git reset --json /path/to/workspace
```

Reset and cleanup announce discovery before probing repositories. Interactive terminals
show discovery elapsed time; redirected output includes periodic discovery updates.
The final elapsed time includes discovery as well as execution.

A checkout or directory inside it selects its primary checkout. Containers recursively
select primary checkouts, stopping at repositories and skipping directory symlinks.
Overlapping scopes are deduplicated. Linked worktrees encountered in containers are
excluded; explicit linked-worktree reset targets are refused. Invalid scope prevents
all resets. `--list` is inspection only and does not fetch.

The existing reset policy is preserved: tracked/index changes, dirty submodules,
unfinished Git operations, masked tracked paths, and untracked/ignored paths that
collide with either the fetched target or local target branch cause refusal.
The remote's advertised HEAD determines the default branch. Fetch mappings must
resolve it unambiguously, match the fetched tip, and never replace local branches.
Hooks are followed by branch, commit, and tracked-file verification. Other branches,
linked worktree files, and noncolliding local files remain intact. Replaced target
branch tips are retained under `refs/home-reset-backups/`. Detached-HEAD commits
receive no additional backup. Successful runs summarize saved recovery refs;
`--verbose` shows individual recovery commands. Failed checkouts show their recovery
commands by default, and JSON always includes the exact backup refs.

The measured existing default remains 32 workers, bounded by checkout count;
this migration introduces no new performance claim. `--jobs` accepts 1–32,
`--attempts` accepts 1–10 total attempts, and `--timeout` accepts 1–86400 seconds
per attempt. Transient failures use bounded backoff. Authentication, locks, and
safety refusals are not retried. There is no transaction or rollback across checkouts:
fetches, backups, switches, and upstream changes may persist after later failure.

## Clean up linked worktrees

The default is an exact removal plan. `--apply` executes eligible removals without
an interactive question. A container scope selects registered linked worktrees
beneath it. A primary checkout selects all its registered linked worktrees, including
paths outside the checkout directory; those full paths are shown. An explicit linked
worktree selects only itself. Primary checkouts always remain protected.

Cleanup protects tracked changes, untracked files, masked tracked paths, ongoing Git
operations, locked worktrees, and unverifiable ownership. These are signs of work
that must stay in place. These worktrees are reported as `protected`; leaving active
work in place is a successful cleanup outcome, not an error. Ignored files do not block an otherwise idle worktree:
`--apply` removes them with the worktree. Preview reports their counts and full
worktree paths. Ignored files can include local configuration and secrets; inspect
the preview before applying a container-wide cleanup.

### Recover committed files

Committed history does not have to be pushed before cleanup. Application first
atomically saves HEAD as `refs/workctl/cleanup/<HEAD>` in the protected primary
repository and verifies that ref. Preview never writes it. Local branches also
remain. Cleanup works offline and never pushes commits. To restore committed files:

```sh
git -C /path/to/primary worktree add --detach /path/to/restored refs/workctl/cleanup/<HEAD>
```

Recovery refs preserve committed history, not discarded ignored files. A failure to
save or verify the recovery ref blocks deletion. These refs are intentional retained
deliverables, not temporary files.

### Strict policy and explicit discard

Use `--strict` for the original publication policy: all local files including ignored
files are protected, and every reachable commit must be proven published against a
fresh isolated fetch of current remote heads and tags. Cached tracking refs are not
proof. Source URL rewrite rules, shallow repositories, case-colliding refs, and
commit-only filtered evidence fetches are supported without changing source refs or
shallow boundaries. Remote verification remains bounded by `--timeout`.
In strict mode, repeat `--discard-ignored PATH` to authorize ignored-file loss and
`--preserve-commits PATH` to use local recovery instead of remote publication at an
exact selected worktree.

For a worktree whose **all local files** may be lost, explicit per-path approval is
still required:

```sh
workctl git worktree clean --apply \
  --discard-local /path/to/work/feature /path/to/work
```

### Removal safeguards and outcomes

No option authorizes removing main checkouts, locks, missing paths, nested
repositories, populated submodule paths, private submodule object stores, or
unverifiable registrations/ownership.
Empty or absent uninitialized gitlink paths are supported, including gitlinks with
no `.gitmodules` mapping. Normal cleanup and ignored-only approval use native
`git worktree remove`; `--force` is used only with `--discard-local`. Cleanup never
retries removal.
`--timeout` bounds each candidate's inspection, remote verification, removal, and
final verification together. Discovery Git probes have independent 15-second bounds.

Application rechecks files, locks, topology, HEAD, and registration before removal.
Strict mode also rechecks publication and then local state after remote I/O. Success requires both the
path and its Git registration to be absent. Outcomes are `would remove`, `removed`,
`protected`, `blocked`, or `interrupted`. Active local work stays protected with exit 0;
actual safety blockers or deletion errors cause exit 1.
A removed worktree's local branch is retained. Stale registrations with missing
filesystem paths remain blockers, rather than being pruned speculatively.

### Coordination and cancellation

Both operations share process supervision and common-directory locks. These locks
coordinate workctl and the former reset runner, not editors or unrelated Git commands;
preview may create the persistent `repo-batch.lock` coordination file in the common
Git directory. It does not change refs, tracked files, or worktree registrations.
Publication checks access the remote and use automatically removed temporary object
stores. The coordination file remains in place so concurrent callers lock the same inode;
run while repositories are otherwise idle. Cancellation/deadlines terminate process
groups before locks are released. Descendants inherit the lock so forced supervisor
termination cannot release it while they run. Interrupted Git writes may leave native
locks requiring investigation; workctl never removes those locks.

### Cleanup concurrency and benchmarks

Cleanup runs up to four worktrees concurrently by default; `--jobs N` accepts 1–32.
Worktrees from the same repository share one exclusive coordination lock for the
invocation. Recovery-ref writes are serialized within each repository; inspections
and native Git removals run concurrently. Other workctl invocations remain excluded
until the batch finishes. Each removal still performs its own final safety inspection.
JSON reports the worker count and per-worktree inspection, recovery, and removal
timings; `--verbose` prints those timings. JSON results retain discovery order even
when worktrees finish out of order.

Measure complete cleanup on dependency-heavy disposable worktrees with:

```sh
python3 examples/cleanup_benchmark.py --candidate target/release/workctl \
  --baseline /path/to/previous/workctl --output /tmp/cleanup-benchmark.json
```

The benchmark compares worker counts on fresh worktrees sharing one repository and
verifies recovery refs, protected local work, primary checkout state, and removal.

## Output and JSON

Human output names the operation, scope/counts, current activity, and final outcome.
Indicatif updates an interactive progress row; permanent failures remain visible above
it. Redirected stderr uses plain progress lines at most once every 15 seconds.
Full paths remain visible, with terminal wrapping rather than truncation. Durations
are readable. Status color includes the equivalent words, respects `NO_COLOR`,
`CLICOLOR=0`, and `TERM=dumb`, and is disabled on redirected streams.
Stdout contains requested results; stderr contains progress and diagnostics.

Detailed Git logs require `--verbose` or an actionable failure. Reset retains private
diagnostics on failure/interruption or `--keep-logs`, including `results.json` and
`excluded-worktrees.json`. Successful default runs remove them. Cleanup's temporary
publication repositories and captures are removed automatically.

`--json` works before or after a subcommand and produces one undecorated JSON document
on stdout. Reset and doctor use schema version 1; cleanup uses version 2. Each has `schema_version`, `operation`, and `status`; operational
errors have `error`. Reset and cleanup include scope, per-target `results`, and summary
counts with `elapsed_seconds`. Reset additionally reports excluded worktrees,
recovery refs, attempts, and retained diagnostics. Cleanup reports protected primary
checkouts, discard decisions, and publication/local-data evidence for eligible targets.
Cleanup version 2 reports `policy` (`pragmatic` or `strict`) and `protected_worktrees`
in the summary. Pragmatic local-work skips have target status `protected` and do not
make the command fail. Strict mode retains `blocked` and exit 1 for these refusals.
Evidence includes planned/saved recovery refs and explicitly distinguishes local
recovery from verified remote publication. Other pre-removal refusals use `blocked`. After removal starts,
`removed` requires verified absence of both the path and registration; `failed` means
one remains; `unverified` means the final state could not be checked. Inspected evidence
is retained for these outcomes. A verified removal following a Git error still counts
as removed, carries a reason, and exits unsuccessfully. Summary `failed` and `unverified`
count those outcomes; `errors` also includes verified removals with Git errors. These
errors produce operation status `failed` and exit 1 (SIGINT remains exit 130).
Consumers of cleanup version 1 must handle these statuses and summary fields before
accepting version 2. No failed deletion is retried.
Per-target `path` is a display string; `path_bytes` is the exact Unix path as an array
of byte values, preserving non-UTF-8 filenames. Help, version, and usage errors follow
Clap conventions even when `--json` is present. Doctor checks Git 2.36+, the executable,
temporary storage, and relevant process/lock capabilities without repairs. It supports
standalone executables and does not certify managed installation or shell configuration;
use `config-tools check-install` for managed installation checks.
