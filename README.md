# config

Personal configuration and agent-skill registry.

## Reset repositories

`reset_to_origin` is a standalone Rust executable backed by the installed Git.
Install it with `cargo run -- install`; production execution needs neither Python
nor zsh. The same compiled multicall binary provides `config-tools` and
`reset_to_origin`, with one reset implementation in `src/reset/`.

```sh
reset_to_origin
reset_to_origin ~/atlassian ~/oss ~/src ~/stable
reset_to_origin --list .
reset_to_origin --jobs 4 --attempts 3 --timeout 300 /path/to/workspace
reset_to_origin --remote upstream --branch release /path/to/repository
```

The CLI uses Clap's derive API for generated help, version information, typed
option validation, and typo suggestions. Use `-h` for concise help, `--help` for
the full contract and examples, and `-V` or `--version` for the package version.
Options accept either `--jobs 4` or `--jobs=4`; `-j` and `-v` are available for
jobs and verbose output. Use `--` before paths that begin with a dash. Repeated
options use the last value. Invalid options fail before discovery or mutation.

No paths means `.`. Positional arguments are filesystem paths, never remote or
branch names. A repository root or a directory inside it selects that primary
checkout. A container recursively selects primary checkouts beneath it. There
are no predefined roots, branch-name guesses, or build/cache exclusion lists.
Discovery stops at repositories, skips Git metadata, and does not follow directory
symlinks encountered during traversal. An explicitly supplied symlink is resolved
normally. Linked worktrees found in containers are reported and skipped; selecting
one explicitly fails. Overlapping inputs run each checkout once. All inputs and
discovery must succeed before any reset starts.

The runner inherits the supervised parallel model of `home_reset_to_origin`:
four workers across all supplied roots, common-directory locks, per-attempt
deadlines, bounded transient retries, preserved local branches/worktrees, and
recoverable target-branch tips. One repository uses the same runner and safety
checks as a batch. `--attempts` means total attempts, default three; `--timeout`
is seconds for the entire attempt, default 300. Limits are 32 workers, 10 attempts,
and 86400 seconds per attempt. Network retries use capped exponential backoff with jitter.
Authentication, Git locks, and safety refusals are not retried.

The operation refuses tracked/index changes, dirty submodules, unfinished Git
operations, tracked paths marked assume-unchanged or skip-worktree, and untracked
or ignored paths that collide with the fetched target or existing local target branch. Fetch is synchronous; the advertised remote
HEAD supplies the default branch. Existing fetch mappings are respected, and the
target must map unambiguously into remote-tracking refs and match the fetched
tip. Fetch mappings that could replace local branches are refused. It switches to
the target branch, checks files, branch identity, and branch-tip changes again
after checkout hooks, and resets to that verified commit. Switch/reset do not recursively update submodules.
Other local branches and noncolliding untracked/ignored files are preserved.
Replaced target-branch tips are saved under `refs/home-reset-backups/`; logs show
`git branch recovered-work <backup-ref>`. Detached-HEAD commits receive no
additional backup.

This is not a transaction across repositories. Fetches, backups, a branch switch,
or upstream changes may persist after a later failure. Logs record stages and
partial progress. Common-directory locks coordinate this runner, not unrelated
Git commands or editors; run while repositories are otherwise idle. Timeouts and
SIGINT/SIGTERM kill worker process groups before releasing repository locks.
Interrupted Git writes may leave locks that require investigation; this tool
never deletes them. The lock descriptor is inherited by workers and descendants
so a forcibly terminated supervisor cannot release it while they still hold it.

Single-repository output names the fetched target and recovery ref. Batches show
scope counts, restrained progress, immediate wrapped failures, and a final summary;
successful repositories do not emit individual rows. `--verbose` prints sanitized
full logs in discovery order. Terminal colors respect `NO_COLOR` and `TERM=dumb`;
redirected output is plain text. A private temporary log directory contains each
attempt log, atomic worker status records, `results.json`, and `excluded-worktrees.json`.
Worker progress and results come from structured records; Git and hook text stays
in diagnostic logs. Incomplete worker exits fail explicitly. Exit codes are 0 for success
(including empty discovery), 1 for operational failures, 2 for invalid usage,
and 130 for interruption.

The old shell reset commands and Python runners have been removed. Use a new
shell after installation so stale function definitions cannot shadow the binary.
Replace `home_reset_to_origin` with explicit paths, replace positional remote/branch
arguments with `--remote`/`--branch`, and replace `--retries` with `--attempts`.
There are no single/multi modes, branch-pruning options, or implicit fetch-config
repairs. Automatic case-conflict repair is intentionally excluded: conflicting
refs fail without adding exclusions or deleting tracking refs. Existing exclusions
remain in effect and may cause a target refusal.

Checks and benchmarks only reset disposable local repositories:

```sh
cargo test --test reset_cli
cargo run -- check
cargo build --release
cargo run --release --example reset_benchmark -- \
  --binary target/release/config-tools \
  --baseline /path/to/old/zsh \
  --output /path/to/benchmark.json
```

The required network E2E lane is `cargo test --locked --test reset_e2e -- --nocapture`.
It starts a real loopback Git daemon, drives the standalone CLI name, and verifies
remote commits, recovery refs, preservation, and dirty-repository refusal in a
multi-root batch. It also runs automatically under `cargo test`, `check`, and
staged pre-commit checks. `.github/workflows/reset-e2e.yml` runs the same lane on
Linux and macOS for pushes and pull requests. Missing Git, unavailable loopback
networking, or failed daemon readiness fail the lane; there is no skip mode.
Set `RESET_E2E_ARTIFACT_DIR` to preserve command transcripts, daemon logs, and
CLI result logs. Otherwise a failing test retains its temporary workspace and
prints its path. The daemon is stopped on success and assertion failure; successful
runs remove their repositories. Existing `reset_cli` tests cover process timeout,
cancellation, and transient-failure classification separately.

The benchmark compares the old **supervised parallel** home runner with Rust using
one and four workers on twelve clones, three rotated trials, a controlled 250 ms
fetch delay, and verified final HEADs. It also measures Rust help startup and
repository discovery. This measures controlled local latency, not live remote
performance. Keep an isolated baseline copy before removing the old runner.

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
- `install`: intentional home-directory mutation for `~/.agents`, custom `~/.codex/skills` and `~/.claude/skills` symlinks, Codex config flag repair, `~/.local/bin/config-tools` and `~/.local/bin/reset_to_origin`. Codex itself is installed and updated through its official distribution; this installer does not create a Codex launcher.
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
- `~/.local/bin/config-tools` and `~/.local/bin/reset_to_origin` are runnable copies of the Rust multicall executable.
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
