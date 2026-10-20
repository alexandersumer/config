# Git operations

[workctl guide](workctl.md) · [README](../README.md)

Reference for checkout resets, linked-worktree cleanup, recovery, and Git-specific
JSON fields. For installation and everyday commands, start with the
[workctl guide](workctl.md).

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
The remote's advertised HEAD determines the default branch. Reset fetches only
that branch into its configured tracking ref, without pruning or updating unrelated
branches or tags. Fetch mappings must resolve it unambiguously and never replace
local branches. Symbolic target refs and case aliases of the selected refs on
case-insensitive filesystems are refused before fetch; unrelated case-colliding
refs remain untouched. The commit returned by this fetch is authoritative, even
when the branch moved after discovery. Fresh branch-specific `FETCH_HEAD` evidence
must match the tracking ref before reset. Cached fetch evidence cannot authorize
a reset. Sharing a directory with untracked or ignored files is safe when the
individual paths do not collide; nested repositories and symlinks remain protected.
Hooks are followed by branch, commit, and tracked-file verification. Other branches,
linked worktree files, and noncolliding local files remain intact. Replaced target
branch tips are retained under `refs/home-reset-backups/`. Detached-HEAD commits
receive no additional backup. Successful runs summarize saved recovery refs;
`--verbose` shows individual recovery commands. Failed checkouts show their recovery
commands by default, and JSON always includes the exact backup refs.

Reset uses up to 32 workers by default, bounded by checkout count. `--jobs` accepts 1–32,
`--attempts` accepts 1–10 total attempts, and `--timeout` accepts 1–86400 seconds
per attempt. Transient failures use bounded backoff. Authentication, locks, and
safety refusals are not retried. Failures show the number of attempts and a bounded,
readable error summary; retained logs and JSON contain the complete diagnostic.
Persistent connection failures remain failures after the retry limit. Use fewer
workers with `--jobs N` on constrained networks. There is no transaction or rollback across checkouts:
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

## Git diagnostics and JSON

Detailed Git logs require `--verbose` or an actionable failure. Reset retains private
diagnostics on failure, interruption, or `--keep-logs`, including `results.json`
and `excluded-worktrees.json`. Successful default runs remove them. Cleanup removes
its temporary publication repositories and captures automatically.

Reset and cleanup include scope, per-target `results`, and summary
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
of byte values, preserving non-UTF-8 filenames.
