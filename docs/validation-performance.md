# Validation performance evidence

Measurements were made on October 29, 2026 (UTC). This report accompanies the execution
changes in `src/validation.rs` and `.github/workflows/reset-e2e.yml`. The compact
sample ledger records failed experiments as well as successful runs. Failed runs
are evidence of rejected configurations, not successful latency improvements.

## Protocol

Local time starts immediately before invoking `sh .githooks/pre-commit` and ends
when that process exits. It includes the outer Cargo invocation, index checkout,
fresh snapshot target, compilation, all Rust suites, recursive snapshot checks,
registry validation, installation checks, and skill regressions. Baseline and
candidate invocations alternate on the same 14-core Apple Silicon workstation.
The final matched timing cohort sets `RUST_TEST_THREADS=4` on both sides. This
fixes per-suite resources without changing any test selection or assertion.
Ordinary unset-environment runs are retained separately, including failures.
The final production commit also receives an ordinary full-hook invocation;
its safe runner policy caps each suite at four workers in either environment. Snapshot targets are cold every time. The Cargo
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
fixture. A subsequent serial control exposed an auth fixture-slot deadlock;
its cancelled invocation and sampled blocked thread stacks are retained. A failed early exit is not time to complete successful validation.
The final matched control is `68a2be3b09bd67e1acd62787cf53acd2e03df11c`:
main plus the same two deterministic closed-output fixtures, registration
coordination fix, two real Git coordination regressions, and auth fixture lifetime
corrections as the candidate.
It retains the original serial Cargo test execution and serial 32-case matrix.
These common corrections preserve assertions and deadlines and prevent known
races from deciding the performance comparison.

The fixed candidate is `e214c1298b6caa41198e9afb62d138dbd809fe72`.
Both sides invoke the complete tracked hook with the same four-worker setting.
The unmodified ordinary serial control cannot supply a reliable successful tail:
its retained native-concurrency invocations include early failures. Controlled
results must not be presented as proof of a fleet-wide ordinary P90 reduction.
Both snapshots include the same full repository and skill registry. The candidate
adds its runner checks and divides the existing matrix across harness tests;
it executes at least the control's validation. Earlier exploratory cohorts remain in the ledger with their own fixed SHAs;
they are not pooled into the final performance statistics.
The production commit is verified again after documentation and consolidation.

There is no file selection to benchmark around. The mixed implementation changes
Rust code, tests, workflow wiring, and documentation together. The staged-index
contract tests exercise mixed staged/unstaged content and untracked skills.
Shared path dependencies, source changes, stale lockfiles, missing/cold caches,
backdated stale build outputs, and concurrent execution receive explicit fixture
or hosted checks. These are correctness probes, not additional ordinary latency
samples. Tests requiring live credentials remain outside the original gate.

## Final local results

| Pair | Control | Candidate | Successful reduction |
| --- | ---: | ---: | ---: |
| 1 | 643.34s | 509.19s | 20.9% |
| 2 | 673.00s | 508.26s | 24.5% |
| 3 | 585.60s | 491.74s | 16.0% |

All six full-hook invocations passed. Median waiting time fell from **643.34
to 508.26 seconds (21.0%)**; P90 fell from **673.00
to 509.19 seconds (24.3%)**. Each side has three samples.
Mean observed child CPU time was 1092.7 versus 1285.6 seconds.
The three pairs used B,C / C,B / B,C order. Outer targets were cold in the first
pair and warm only through normal prior invocations thereafter. Snapshot targets
were new every time. No other local test or profiling workload ran concurrently.

The local 30% target was **not achieved**. These controlled results use the same
four-worker setting on both commits; they are not proof of ordinary fleet-wide
median or P90 improvement. The exact production commit separately receives a full
hook invocation with `RUST_TEST_THREADS` unset. Existing native-default baseline
failures and explicitly terminated deadlock experiments remain in the ledger.

The first pair shows the critical path: the recursive snapshot suite fell from
413.47 to 331.71 seconds. Serial outer lib/reset/loopback/worktree execution took
152.78 seconds; the candidate overlapped the same contracts in 101.68 seconds.
Its new runner fixtures took 9.66 seconds and are included. The remaining time
includes outer bootstrap, fresh-snapshot compilation, formatting/checking, registry
and skill/install validation, and cleanup. No required work was moved outside
the timer. The recursive suite remains the largest cost.

Further resource increases failed original deadlines; three workers per heavy
suite and serial Cargo execution were slower. Compiler bookkeeping and process
spawn experiments showed no gain, while the unchanged skill/install component
took only 6.28 seconds. Retaining fresh isolated snapshots and every original
inspection, assertion, and threshold limits the proven local gain. Larger gains
would require a new design with separately verified invalidation and isolation.

## Final hosted results

| Pair | Control wait | Candidate wait | Outcome |
| --- | ---: | ---: | --- |
| 1 | [176s](https://github.com/alexandersumer/config/actions/runs/40855880432) | [104s](https://github.com/alexandersumer/config/actions/runs/40855888588) | success / success |
| 2 | [116s](https://github.com/alexandersumer/config/actions/runs/40856307513) | [110s](https://github.com/alexandersumer/config/actions/runs/40856311593) | success / success |
| 3 | [246s](https://github.com/alexandersumer/config/actions/runs/40856634329) | [76s](https://github.com/alexandersumer/config/actions/runs/40856641246) | success / success |
| 4 | [227s](https://github.com/alexandersumer/config/actions/runs/40857313200) | [116s](https://github.com/alexandersumer/config/actions/runs/40857317641) | success / success |
| 5 | [135s](https://github.com/alexandersumer/config/actions/runs/40857872718) | [101s](https://github.com/alexandersumer/config/actions/runs/40857876908) | failure / success |

All five candidate workflows passed all six Linux/macOS jobs. Four controls
passed. Control pair five failed the unchanged
`worker_crash_reports_incomplete_status_without_fabricating_success` assertion on
macOS: it reported `worker group still active after termination` rather than the
expected missing terminal-status diagnostic. Its 135-second failed exit remains
in the ledger and is not successful completed validation. No run was retried.

For the four pairs in which both sides passed, median waiting time fell from
**201.5 to 107 seconds (46.9%)**, and P90 from **246 to 116 seconds (52.8%)**.
The all-attempt observations are 176/246 seconds median/P90 for the control and
104/116 for the candidate, with failure rates 1/5 and 0/5 respectively. A failed
early exit must not be interpreted as a successful validation latency.

Observed runner wall time summed across all jobs averaged 293.25 seconds for the
four passing controls and 391.5 seconds for their matched candidates: **33.5% more
runner time**. Across all ten attempts it totals 3,348 runner seconds. Reliable
billed usage was unavailable. Cold caches, stable-toolchain refresh, and artifacts
are included. The first candidate run had six genuine cache misses; subsequent
caches were populated only by normal successful validation. Each of the next
four workflows restored all six job caches, as verified from its full log.

The latest-finishing job was macOS in every control. For candidates, macOS
worktree was last in three pairs, macOS reset in one, and Linux reset in one.
In pair one the critical contract step fell from 163 to 86 seconds; complete
waiting time fell from 176 to 104 seconds after queue/setup/cache/artifact/post
work. In pair two the same step took 106 versus 87 seconds and complete wait
was 116 versus 110, demonstrating why test runtime alone overstates the gain.

The complete retained hosted ledger contains 128 completed workflows: 117
succeeded and 11 failed, totaling 40,288 observed runner seconds (11.19 runner
hours). This includes historical controls and exploratory configurations, not just
the final cohort, and is not an account billing total. Publication verification
runs occur after this report is committed and are retained separately.

GitHub API connection failures interrupted the collector during pair three.
Collection resumed using the same already-dispatched IDs; only read calls were
retried. The API failures and original coordinator exit are retained.

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
The desktop budget is eight in total, divided between the two long suites,
with at most four workers per executable;
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

Two simultaneous real runner invocations also share a cold Cargo fixture target.
A rendezvous proves their test processes overlap; both finish successfully with
distinct log directories and only their own output. This exercises Cargo build
locking and log isolation without a prebuilt fixture or cached result.

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
must fail their respective guards, then pass after exact restoration. A real-shell
closed-output regression changes stdout from writable to broken while two children
are active. All three selected children must still start and finish, and the runner
must report failure. Replacing its checked progress write with a panic fails this
regression; exact restoration passes.

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
The fault fixture calls its Python barrier only at the three intended interception
points and delegates other commands directly to real Git. Earlier interpreter
startup overhead made one control run miss its original ten-second barrier; that
failed run remains recorded. The corrected five-mode fixture passed 20 consecutive
repetitions (100 cases), including owned, unknown, idle, repeated, and disappearing
commondir errors, with unchanged ten/15-second deadlines. Disabling the product
revalidation fails the fixture; restoration passes. No test or workflow is retried. A real
Git fixture requires two native removals to overlap and detects verification
before they become quiescent. Replacing the exclusive guard with a shared guard
fails that check; exact restoration passes. The original eight-worker shared-head
recovery and two-worker cancellation/owner-loss tests still pass unchanged.

A native 14-thread serial-control run exposed a pre-existing auth fixture deadlock.
Four tests retained completed fixtures while acquiring another slot from a pool
of four. Sampling showed those tests blocked in `FixtureSlot::acquire` with no
provider children. The invocation was explicitly terminated and retained as a
cancellation, not treated as a completed latency sample. Five explicit fixture
drops release completed scenarios before starting the next. A thread-local guard
rejects nested fixture ownership before acquiring the pool mutex, and its slot
cannot move between threads. Pool capacity remains four. All original 64 auth
test bodies are identical apart from those five drops and formatting; a new
ownership regression checks prompt rejection without leaking or poisoning slots.
All 65 auth tests pass at native 14-thread concurrency. Removing one drop makes
the original service scenario fail promptly; exact restoration passes.

One recovery test contained 32 independent scenarios in a serial loop. Its four
policy contexts now have separate harness tests, each executing the unchanged
eight-kind assertion body. All 32 cases remain, including strict/local modes and
new/existing recovery pins. The embedded mutator scripts are byte-identical, and
the moved body differs only in indentation and formatting commas. The same
component workload took 54.02 seconds before and 16.29 seconds after, including
recompilation; this is a component result, not a full-hook latency claim.

## Counts and gates

On successful runs of unmodified main `547822a`, hosted Cargo targets execute 220 tests on Linux
and 221 on macOS. The corrected serial control executes 223 and 224 respectively;
the three extra tests cover shared registration coordination and fixture ownership. These are counts for successful complete runs. The candidate
executes 231 and 232 respectively: two new runner unit tests, three runner
integration tests, and three extra harness entries from splitting the existing
32 recovery cases. Every original assertion remains. Local complete hooks add
the two pre-commit integration tests: 226 top-level tests for the corrected control
and 234 for the candidate, plus their recursive checks and expected-failure
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
harness concurrency failed existing timing/cancellation contracts. A final
experiment with seven workers per heavy suite also failed existing cancellation,
lock-lifetime, and discovery deadlines during recursive snapshots; it was rejected.
A durable default-worker assertion now catches that exact policy regression. Those runs
remain in the ledger. A speculative change to Git process spawning produced no
measured gain and was reverted. Reusable repository fixture setup measured about
0.24 seconds per fixture and was not the dominant cost; no repository fixture
cache was introduced. Disabling incremental bookkeeping in a fresh full-test target measured 6.36
versus 6.37 seconds, so compiler settings were left unchanged. Snapshot result
reuse and narrowing the recursive checks
were rejected because they would weaken the validation contract.

Further measured components did not justify additional production changes. The
complete 127-test Git workload took 141.17 seconds cold and 121.75 warm at three
workers per suite, versus 115.19 and 98.99 at four. Native serial Cargo execution
took 133.49 seconds warm. Skill/install regression validation took 6.28 seconds;
its binary-marker scan was not a dominant full-hook cost. Disabling incremental
compilation did not improve cold compilation. A speculative descriptor/spawn
change took 128.81 seconds and failed existing descendant-cleanup assertions; it
was reverted. These are component experiments, not full-hook latency samples.

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
by this change. No deployment is performed. Concurrent hosted cache writers were observed;
simultaneous full local hooks were not benchmarked. Alternate-compiler cache
compatibility is enforced by the exact key, without an additional compiler-install
experiment. Executable stale-dependency probes use shared path packages; Git-sourced
packages follow the same non-registry cleanup rule.
