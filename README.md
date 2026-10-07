# config

Personal configuration and agent-skill registry.

## Parallel home reset

```zsh
home_reset_to_origin --list
home_reset_to_origin --jobs 4 --timeout 300 --retries 3
home_reset_to_origin --root ~/oss --jobs 2
```

Requires Python 3 and zsh on macOS or Linux. The shell entry point delegates to
`zsh/home_reset.py`; `zsh/repo_batch.py` handles bounded workers, locks, deadlines,
and retries without knowing Git reset policy.

The default roots are `~/atlassian`, `~/oss`, `~/src`, and `~/stable`.
`HOME_RESET_TO_ORIGIN_ROOTS` can supply a shell-quoted list of roots. Discovery
walks folders in sorted order, skips build/cache directories and directory
symlinks, and stops at repositories unless `--include-nested` is set. Linked
worktrees are excluded before fetching: each primary checkout is reset once,
while linked branches and files are left alone. A separate Git directory does
not make a primary checkout a linked worktree. `--list`
uses the same discovery as execution. `--all-home` expands discovery to the home
directory while excluding personal/system folders. Roots run in order, with up
to four concurrent repositories inside each root. Jobs sharing a Git common
directory serialize; overlapping batch invocations refuse a busy repository.

The default output shows one live progress line per workspace in a terminal,
with failures reported immediately using workspace-relative names. Successful
repositories do not generate individual rows. Green marks success, red marks
failure, and yellow marks skipped worktrees or cancellation; `NO_COLOR`,
`TERM=dumb`, and redirected output disable colors. Redirected output uses plain
progress lines only when the completed count changes, plus one wait notice per
slow active repository. `--verbose` prints full Git logs in traversal order
after each workspace. All attempts are retained in each repository log,
with a final summary, `results.json`, and `excluded-worktrees.json`
in the printed temporary log directory. Logs may contain private remote URLs;
the directory is private to the current user and remains until cleaned up.

`--retries` retains its legacy meaning of total attempts, with zero treated as
one, and a maximum of ten. Only recognized temporary network failures and
timeouts retry. Authentication errors, dirty files, and Git lock errors fail
promptly. Retry delays double with jitter and cap near 30 seconds. `--timeout`
is a deadline for the entire attempt, including hooks and Git subprocesses.
Timeouts and Ctrl-C kill each worker's process group before releasing its
repository lock. This is a hard stop, so interrupted Git writes can leave locks
that the runner reports rather than deleting. Genuine signal permission errors
remain failures. Any failed repository yields exit 1; cancellation yields 130.

Home reset now always fetches synchronously and preserves local branches and
linked worktrees instead of pruning them. It refuses dirty tracked files,
unfinished Git operations, and untracked/ignored paths that would be overwritten
by either the local default branch or the fetched target. It disables recursive
submodule checkout/reset and checks again after switching branches. Before
replacing an existing default-branch tip, it saves a ref under
`refs/home-reset-backups/<timestamp>/<branch>`. Recover a saved tip with
`git branch recovered-work <printed-backup-ref>`.

This still deliberately switches to the remote default branch and resets its
tracked tree. It is not an atomic transaction across repositories. The lock
coordinates this batch runner, not editors or unrelated Git commands; run it
while repositories are otherwise idle. A forced kill or machine failure can
interrupt Git and leave a lock requiring investigation; the runner never deletes
Git locks automatically. The shared reset helper now propagates failed fetches
without ref/lock repair or hidden background fetches. Branch pruning preserves
every branch checked out in a worktree, including the canonical checkout, and
only deletes eligible unused branches. Single-repository reset still prunes
unused branches unless `--no-prune` is supplied. All bulk retry paths use the
same network-failure classifier.

Checks and a reproducible benchmark use disposable local repositories:

```sh
python3 tests/test_home_reset.py
python3 tests/benchmark_home_reset.py
cargo run -- test-validate
```

The benchmark compares the original script at the recorded pre-change revision
with one, two, and four supervised workers on twelve clones. It waits for the original
script's detached fetches, uses a controlled 250 ms delay per fetch, verifies
every final HEAD, and writes three samples per configuration to
`tests/home-reset-benchmark.json`. It measures controlled latency; live remote
performance depends on network and server limits.

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
- `install`: intentional home-directory mutation for `~/.agents`, custom `~/.codex/skills` and `~/.claude/skills` symlinks, Codex config flag repair, `~/.local/bin/config-tools`. Codex itself is installed and updated through its official distribution; this installer does not create a Codex launcher.
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
- `~/.local/bin/config-tools` is a runnable copy of the config helper.
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

## Resetting workspace repositories

`home_reset_to_origin` processes canonical repositories with four supervised workers, preserving linked worktrees and local branches. Use `--list` to inspect discovery without resetting anything. Fetch failures stop that repository before reset.

For remote branch names that differ only by case on a case-insensitive filesystem, use:

```bash
home_reset_to_origin --root ~/atlassian/convo-ai --resolve-case-conflicts
```

Recovery changes only the checkout's Git configuration and remote-tracking references. It keeps the remote default branch, an explicitly requested target, local branch names, and local upstream dependencies. It refuses recovery when multiple colliding branches are protected or the fetch mapping is customized. Otherwise it keeps the lowercase spelling when available and excludes the other spellings with exact negative fetch refspecs. Existing tracking tips are saved under `refs/home-reset-backups/case-conflicts/` before removal. Remote branches and linked worktree contents are untouched. Recovery is unnecessary on case-sensitive filesystems or with reftable storage; tag collisions still require manual repair.

Exclusions persist for ordinary future fetches. Every run reports them as warnings and records them in `results.json`; the final status distinguishes completion with fetch exclusions from unrestricted completion. To undo one exclusion after its remote collision is resolved:

```bash
git -C ~/atlassian/convo-ai config --local --fixed-value --unset-all remote.origin.fetch '^refs/heads/EXCLUDED_BRANCH'
git -C ~/atlassian/convo-ai fetch --prune origin
```
