# Validation performance evidence

Measurements were made on October 29, 2026. This report accompanies the execution
changes in `src/validation.rs` and `.github/workflows/reset-e2e.yml`. The compact
sample ledger records failed experiments as well as successful runs. Failed runs
are evidence of rejected configurations, not successful latency improvements.

## Protocol

Local time starts immediately before invoking `sh .githooks/pre-commit` and ends
when that process exits. It includes the outer Cargo invocation, index checkout,
fresh snapshot target, compilation, all Rust suites, recursive snapshot checks,
registry validation, installation checks, and skill regressions. Baseline and
candidate invocations alternate on the same Apple Silicon workstation, with
`RUST_TEST_THREADS` unset for ordinary invocation measurements. Earlier controlled
experiments set it to four for both and are reported separately. Snapshot targets are cold every time. The Cargo
registry is ordinarily warm; the candidate's first outer target is cold, and
subsequent outer targets become warm through normal invocations. No target or
result is copied into a snapshot.

Hosted waiting time starts at the provider's run `createdAt` and ends at the last
job's completion. All matrix jobs, queueing, setup, cache operations, artifacts,
and post steps count. Each matched pair runs its baseline and candidate
concurrently; pairs are sequential. The first candidate run has genuine cache
misses, and subsequent runs reuse caches written by successful normal runs.
No workflow or test is retried automatically. Provider API read retries only
retrieve the same run; they do not execute validation again.

P90 uses nearest rank. With five hosted samples it is the maximum, and with three
local samples it is also the maximum. These samples characterize this experiment,
not fleet-wide tail latency. Runner seconds are the sum of observed job wall
times; the provider timing endpoint does not expose reliable billed compute for
these runs.

## Fixed controls and representative workload

The unmodified main control is `547822a991def3c71583690c5e161661f41346a0`.
Its ordinary full-hook cohort retains failures in the original closed-stderr
fixture. A failed early exit is not time to complete successful validation.
The final matched control is `53170b82018548a4dde9eebbf4a873deb7a06e94`:
main plus the same two deterministic closed-output fixtures, registration
coordination fix, and two real Git coordination regressions as the candidate.
It retains the original serial Cargo test execution and serial 32-case matrix.
These common corrections preserve assertions and deadlines and prevent known
races from deciding the performance comparison.

The fixed candidate is `32bb166a242869ad9f58af4df17693d6c4c57a5e`.
Both sides invoke the complete tracked hook with ordinary thread defaults.
Both snapshots include the same full repository and skill registry. The candidate
adds its runner checks and divides the existing matrix across harness tests;
it executes at least the control's validation. The earlier hosted cohort at
`1c6e8c9ca6d4c2b93254e534c7f08daee8b64cf4` has the same execution policy;
the final candidate additionally rejects unknown or missing Cargo record types.
The production commit is verified again after documentation and consolidation.

There is no file selection to benchmark around. The mixed implementation changes
Rust code, tests, workflow wiring, and documentation together. The staged-index
contract tests exercise mixed staged/unstaged content and untracked skills.
Shared path dependencies, source changes, stale lockfiles, missing/cold caches,
backdated stale build outputs, and concurrent execution receive explicit fixture
or hosted checks. These are correctness probes, not additional ordinary latency
samples. Tests requiring live credentials remain outside the original gate.

## Correctness boundaries

Cargo builds the complete selected target inventory before execution. A failed build or missing/malformed required artifact metadata rejects the run
before any partial inventory can execute. The real Cargo fixture also fails one
target while other warm artifacts exist, proving that none execute.
Full checks discover all normal Cargo test targets and separately run doctests.
The hosted target union retains lib, reset CLI, loopback E2E, worktree, and auth
contracts on both operating systems, adding the runner regression suite. No diff,
path, branch, or previous test result selects validation.

Hosted auth, reset, and worktree groups run on separate runners on both operating systems.
Locally, auth runs without competing Git suites. Recursive pre-commit tests run alone.
Remaining executables use at most two child processes, starting long Git suites
first. Cargo-building runner fixtures and doctests execute in a final isolated phase.
The desktop budget follows available CPUs, capped at 16 total workers and divided
between the two long suites. Other desktop suites retain a four-worker ceiling;
hosted jobs explicitly retain their original per-suite CPU concurrency. Every
phase is awaited even after another fails. File-backed output avoids pipe
backpressure and interleaved logs. Failures retain diagnostics; hosted logs use
the existing always-uploaded artifact directory.

Hosted Cargo children explicitly inherit the stable toolchain. Cache keys include
OS, architecture, exact compiler identity, manifests,
lockfile, and Cargo configuration, without fallback keys. Local and Git package outputs, including shared path dependencies, are
removed before bootstrap and before cache saving. Thus restored root binaries
cannot replace the current validator. Only immutable registry dependencies remain cached. Cargo still owns dependency invalidation;
test outcomes are never cached. Cache loss or incompatibility causes compilation.
Concurrent cold writers were observed; a winner can save the immutable cache
while other jobs finish their full validation normally.

Staged snapshot construction, Git environment scrubbing, independent target
creation, and installation/fresh-shell contracts retain their original behavior.
Mixed changes receive the same full checks. The original staged-index test still
proves that invalid unstaged Rust and untracked skills are ignored, that invalid
staged skills fail even with a valid working copy, and that local files survive.

The runner regression suite exercises real Cargo fixtures with concurrent test
executables, large diagnostics, an integration failure, a doctest failure, changed
source, a changed shared path dependency, a stale lockfile, and deliberately
backdated invalid root and shared dependency sources after local-package cleanup. It also rejects partial
metadata and invalid worker settings. Process-budget and worker-budget mutations
must fail their respective guards, then pass after exact restoration.

Two closed-output fixtures exposed timing races during hosted measurements. A
help process could write before its parent closed stdout; a delayed parent could
close stderr after the intended progress report had already been emitted. The
fixtures now preclose a real pipe or use an explicit close marker. Their existing
exit-code, escaping, descendant-cleanup, recovery-ref, and filesystem assertions
remain, as do the 15-second termination deadline and existing delays. No product
output or cancellation logic was changed to make these tests pass.

Concurrent local stress exposed a real Git metadata race: a post-removal
`worktree list` could overlap the final native removal deleting `.git/worktrees`.
Each repository now permits concurrent native removals under shared guards and
requires an exclusive guard for post-removal verification. Coordination observes
the same existing deadline and cancellation state. It retains both filesystem and registration verification. An inspection that
observes one of the two measured disappearing-registration errors while an owned
removal overlaps logs that error and repeats the complete ownership/data check
once under an exclusive guard. A JSON counter exposes this revalidation.
It uses the original deadline; unrelated, unowned, and repeated errors fail closed.
A fault-injection fixture proves that data created during the error is preserved.
No test or workflow is retried. A real
Git fixture requires two native removals to overlap and detects verification
before they become quiescent. Replacing the exclusive guard with a shared guard
fails that check; exact restoration passes. The original eight-worker shared-head
recovery and two-worker cancellation/owner-loss tests still pass unchanged.

One recovery test contained 32 independent scenarios in a serial loop. Its four
policy contexts now have separate harness tests, each executing the unchanged
eight-kind assertion body. All 32 cases remain, including strict/local modes and
new/existing recovery pins. The embedded mutator scripts are byte-identical, and
the moved body differs only in indentation and formatting commas. The same
component workload took 54.02 seconds before and 16.29 seconds after, including
recompilation; this is a component result, not a full-hook latency claim.

## Counts and gates

On the current unmodified main, hosted Cargo targets execute 220 tests on Linux
and 221 on macOS. The corrected serial control executes 222 and 223 respectively;
the two extra tests cover the shared registration coordination fix. The candidate
executes 229 and 230 respectively: one new runner unit test, three runner
integration tests, and three extra harness entries from splitting the existing
32 recovery cases. Every original assertion remains. Local complete hooks add
the two pre-commit integration tests: 225 top-level tests for the corrected control
and 232 for the candidate, plus their recursive checks and expected-failure
fixtures. Both recursive full snapshots exclude only the pre-commit test file,
exactly as before, to avoid infinite recursion. No check is moved out of the hook.

Each ordinary hook still performs staged snapshot preparation, compilation,
formatting, all unit/integration/doctests, registry validation, and skill/install
regressions. Hosted coverage retains real Git, loopback protocol, installation,
and fresh-shell contracts on both operating systems. There are no effective
main rules requiring old matrix status names (the provider rules response was
empty); the workflow has no branch-conditioned execution policy. Its main push
uses the same six-job matrix and exact compatibility keys as temporary branches.
A cold main cache rebuilds rather than relying on experiment-branch cache scope.

## Rejected approaches and limits

Unbounded executable concurrency, insufficient isolation, and excessive combined
harness concurrency failed existing timing/cancellation contracts. Those runs
remain in the ledger. A speculative change to Git process spawning produced no
measured gain and was reverted. Reusable repository fixture setup measured about
0.24 seconds per fixture and was not the dominant cost; no repository fixture
cache was introduced. Disabling incremental bookkeeping in a fresh full-test target measured 6.36
versus 6.37 seconds, so compiler settings were left unchanged. Snapshot result
reuse and narrowing the recursive checks
were rejected because they would weaken the validation contract.

Rust's stable-toolchain refresh, Linux shell installation, root compilation,
real Git/loopback work, and artifact handling remain measured work. A retained
slow hosted run spent 67 seconds refreshing an unchanged stable toolchain. The
workflow retains that refresh rather than trusting image residue or a stale
compiler. Earlier local permission failures are retained as exploratory evidence;
qualified local runs grant the OS permissions needed by existing `ps` and
loopback checks.

## Checks outside this experiment

Existing ignored acceptance tests that require real Atlas, TWG, Bitbucket,
Kerberos, shared credentials, network services, or interactive MFA are not enabled
by this change. No deployment is performed.
