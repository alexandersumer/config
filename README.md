# config

Personal configuration and agent-skill registry.

## Workstation CLI

`workctl` is the public workstation tool. Git maintenance is its first domain;
configuration installation and CI remain internal `config-tools` commands.
There are no standalone checkout-reset or worktree-cleanup aliases.

```sh
workctl git reset ~/atlassian ~/oss ~/src ~/stable
workctl git worktree clean ~/atlassian ~/oss ~/src ~/stable
workctl git worktree clean --apply ~/atlassian ~/oss ~/src ~/stable
workctl doctor
workctl completions zsh
```

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

Every public command has Clap-generated help, version information, and examples.
Use `--` before paths beginning with a dash. Git options belong to their operation:
`--jobs` applies to reset, and `--apply` belongs to cleanup. No paths means `.`;
there are no hardcoded workspace roots. Help and completions require no Git access.
Exit codes are 0 for success, 1 for operational failure or blocked cleanup,
2 for invalid usage, and 130 for interruption.

### Checkout reset

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

### Linked-worktree cleanup

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

### Presentation and machine output

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

### Architecture and verification

`src/workctl.rs` owns the public command hierarchy. `src/runtime.rs` owns process
capture, deadlines, cancellation, and group lifetime. `src/presentation.rs` owns shared
rendering and JSON conventions. `src/git_domain/` owns topology, common-directory locks,
publication, and separate reset/cleanup policies. No plugin registry or command DSL
is involved.

```sh
cargo test --locked --lib --test reset_cli --test reset_e2e --test workctl
# Real protocol E2E only (requires Git and loopback networking):
cargo test --locked --test reset_e2e
cargo run -- check
cargo run -- pre-commit
```

Real-Git tests cover dirty files, hooks, backup refs, custom fetch mappings,
publication, deleted remote branches, scope protection, discard decisions, nested
repositories, locks, revalidation, cancellation, deadlines, concurrency, partial
failure, and resulting registrations/filesystem state. Presentation checks cover
narrow/normal terminal widths, redirected progress, JSON, disabled color, and broken
pipes. Installation tests use temporary homes and fresh sh/zsh shells, verify
completion files, and preserve unrelated legacy files. The Git protocol E2E starts a
loopback daemon and checks reset backups, cleanup previews and partial application,
stale remote references, explicit local-data discard, and persisted registrations.
Missing Git or loopback networking fails the lane instead of skipping it.
The CI workflow runs the public CLI
contracts on Linux and macOS; configured CI is distinct from a completed hosted run.

A disposable worker benchmark remains available:

```sh
cargo run --release --example reset_benchmark -- \
  --binary target/release/workctl --output /tmp/reset-worker-benchmark.json
```

## Tooling

Config tooling is implemented in Rust via the `config-tools` binary.

```bash
cargo run -- check
cargo run -- check-codex-skills
cargo run -- check-claude-skills
cargo run -- check-install
cargo run -- prepare
cargo run -- pre-commit
cargo run -- repair-codex-config
cargo run -- validate
cargo run -- test-validate
cargo run -- install
```

Command roles:

- `check`: non-mutating verification for formatting, build, unit tests, skill validation, and regression tests.
- `check-codex-skills`: non-mutating verification that `~/.codex/skills` mirrors custom skills from `.agents/skills`, Codex config has no deprecated/disabled skill-discovery flags, and Codex prompt input sees the managed skills from both this checkout and the home directory.
- `check-claude-skills`: non-mutating verification that `~/.claude/skills` mirrors custom skills from `.agents/skills`, with every managed link resolving to its registry-validated source and no stale managed links left behind.
- `check-install`: non-mutating verification that all managed home config links, Codex skills/config, Claude Code skills, and the managed config-tools binary match this checkout.
- `repair-codex-config`: removes deprecated/disabled Codex feature flags from `~/.codex/config.toml`.
- `prepare`: runs the same verification as `check`.
- `pre-commit`: validates an isolated snapshot of the Git index, so unstaged and untracked work cannot affect the commit checks. Outside Git, runs `prepare` on the supplied checkout. No home installation is required.
- `install`: intentional home-directory mutation for `~/.agents`, custom `~/.codex/skills` and `~/.claude/skills` symlinks, Codex config flag repair, `~/.local/bin/config-tools` and `~/.local/bin/workctl`. Codex itself is installed and updated through its official distribution; this installer does not create a Codex launcher.
- `install-git-hooks`: intentional local Git config mutation for `core.hooksPath`.

## Custom skills

This checkout is the single source of truth for custom skills. One `SKILL.md` per skill is shared by every consumer; install only adds per-consumer symlinks that point back at it:

- Source: `.agents/skills/<name>/SKILL.md`
- Agent runtime: `~/.agents -> <checkout>/.agents`
- Codex link: `~/.codex/skills/<name> -> <checkout>/.agents/skills/<name>`
- Claude Code link: `~/.claude/skills/<name> -> <checkout>/.agents/skills/<name>`

The same discovery rule applies to every consumer: a top-level, non-hidden `.agents/skills/<name>` directory containing `SKILL.md` is linked; hidden directories and directories without `SKILL.md` are skipped. Linking is idempotent, never clobbers an existing non-managed entry, and removes managed links whose source skill no longer exists.

### Codex

Codex v0.131 does not expose custom skills as `/skill-name` slash commands. The `/` menu is for built-in TUI commands such as `/skills` and `/subagents`. Invoke these custom skills with the skill mention surface instead, for example:

```text
$surgical-edit
$review-solo
$review-deep
$git-publish-to-origin
$one-clear-sentence
```

Do not add `register_cmd` to skill front matter. Current Codex ignores that legacy key, so this repo rejects it to avoid implying that custom skills appear as slash commands.

### Claude Code

Claude Code discovers personal skills from `~/.claude/skills/<name>/SKILL.md`, which is the same `SKILL.md` format the registry already validates (`name`, `description`, `allowed-tools`). Install links every custom skill into `~/.claude/skills`, creating the directory if it does not exist. Discoverability is proven deterministically: `check-claude-skills` asserts each `~/.claude/skills/<name>` resolves to its registry-validated source, which is exactly the contract Claude Code's loader requires.

## TWG CLI skills

The tracked `twg*` skills are the workflow layer for the TWG CLI. They teach agents to use live CLI help instead of guessing command grammar, route Jira, Confluence, Bitbucket, and cross-product requests to focused workflows, prefer machine-readable output, and distinguish PATH, authentication, authorization, and command errors.

TWG remains the source of the bundle. Because `~/.agents` links into this checkout, an upstream skill refresh overwrites files here and can remove skills omitted by a newer release. Save existing changes before updating, review the resulting diff, and preserve local OAuth recovery, review-context behavior, and the benchmark helper before publishing. Keep unrelated config changes out of the refresh commit. The updater can also replace managed agent links with TWG-owned copies. Back up those copies before restoring links to this checkout, and preserve unrelated skill directories. Then reconcile the managed Codex and Claude Code links:

```bash
twg update --refresh-skills
cargo run -- validate
cargo run -- install
```

Use these read-only checks when diagnosing access or discovery:

```bash
twg doctor -o json
twg help discover-skills "<intent>" -o json
twg help describe "<command-or-skill>" -o json
cargo run -- check-codex-skills
cargo run -- check-claude-skills
```

Do not copy TWG's full command catalog into `AGENTS.md`: command details change with the CLI, while the installed skills use progressive disclosure and query live help. If a new Codex session does not show the skills, run `cargo run -- install`, verify with `check-codex-skills`, and restart Codex so it rebuilds its skill inventory.

## Git hooks

This config checkout uses Git's native tracked-hooks convention: `.githooks/pre-commit` is tracked, and the local checkout is configured with:

```bash
git config core.hooksPath .githooks
```

Use the Rust helper to configure that for this checkout:

```bash
cargo run -- install-git-hooks
```

The pre-commit hook delegates to Rust:

```bash
cargo run -- pre-commit
```

Git intentionally does not auto-enable arbitrary hooks from a freshly cloned checkout for security reasons. A fresh checkout therefore needs one local setup step (`cargo run -- install-git-hooks`) before hooks will run. After that, hooks are enabled for that checkout.

## Acceptance criteria

Repository changes must pass these checks locally:

```bash
cargo run -- pre-commit
```

The hook runs formatting, compilation, unit and integration tests, skill validation, and regression tests against the staged snapshot. It preserves the working tree and index. Installation regression tests use temporary homes. The pre-commit integration test checks an uninstalled checkout and verifies that invalid skills and failing tests still block commits.

To diagnose the current user's installation, run these explicit checks separately. Installation drift does not block repository commits:

```bash
cargo run -- check-codex-skills
cargo run -- check-claude-skills
cargo run -- check-install
```

For manual verification of install behavior, use a temporary home without touching the real home directory:

```bash
cargo run -- prepare

tmp_home="$(mktemp -d)"
for skill in skill-creator skill-installer; do
  mkdir -p "$tmp_home/.codex/skills/.system/$skill"
  printf 'fixture\n' > "$tmp_home/.codex/skills/.system/$skill/SKILL.md"
done
cargo run -- install --home "$tmp_home"
find "$tmp_home" -maxdepth 4 -type l -exec ls -la {} \;
rm -rf "$tmp_home"
```

Expected symlink behavior:

- `~/.agents` links to this config checkout's `.agents` directory.
- `~/.zsh` links to this config checkout's `zsh` directory.
- `~/.zshrc` links to this config checkout's `zsh/zshrc` file.
- `~/.config/ghostty/config` links to this config checkout's `ghostty/config` file.
- `~/Library/Application Support/com.mitchellh.ghostty/config` is absent so Ghostty loads the managed config only once.
- `~/.config/relay/config.toml` links to this config checkout's `relay/config.toml` file.
- `~/.local/bin/config-tools` and `~/.local/bin/workctl` are runnable copies of the Rust multicall executable.
- Codex runs directly from the official Homebrew installation. The shell prefers `/opt/homebrew/bin` over older `/usr/local/bin` tools; no config-repair wrapper is installed.
- `~/.codex/skills/.system` remains a Codex-owned directory with Codex system skills.
- Each custom top-level `.agents/skills/<name>/SKILL.md` directory links into `~/.codex/skills/<name>` and `~/.claude/skills/<name>`.
- `~/.claude/skills` is created if missing; the directory itself is not symlinked, only the per-skill entries inside it.
- Hidden skill directories and directories without `SKILL.md` are not linked into Codex or Claude Code.
- Existing non-empty files/directories or unrelated symlinks are not replaced automatically.

## Executable-code check

Tracked non-Rust executables should be limited to repository wiring scripts and skill-owned helper scripts that are invoked directly by their skill documentation.

Verify with:

```bash
find . -path './.git' -prune -o -path './target' -prune -o -type f -perm -111 -print | sort
git grep -n -E '^#!|python3|/usr/bin/env|cargo run --quiet --manifest-path' -- ':!README.md'
```

The executable-file check should print only:

```text
./.githooks/pre-commit
```

`.githooks/pre-commit` delegates to Rust and should contain no config-tool logic beyond `cargo run -- pre-commit`.
