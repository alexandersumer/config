# config

Personal configuration and agent-skill registry.

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
- `check-install`: non-mutating verification that all managed home config links, Codex skills/config, Claude Code skills, and managed local launchers match this checkout.
- `repair-codex-config`: removes deprecated/disabled Codex feature flags from `~/.codex/config.toml`.
- `prepare`: runs the same verification as `check`.
- `pre-commit`: safe hook entrypoint; runs `prepare` and `check-install`.
- `install`: intentional home-directory mutation for `~/.agents`, custom `~/.codex/skills` and `~/.claude/skills` symlinks, Codex config flag repair, `~/.local/bin/config-tools`, and a `~/.local/bin/codex` launcher that repairs deprecated flags before delegating to Homebrew Codex.
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

TWG remains the source of the bundle. Refresh it intentionally, review the resulting skill diff, then reconcile the managed Codex and Claude Code links:

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

A change is accepted only if all of these pass locally:

```bash
cargo fmt --check
cargo check
cargo run -- check
cargo run -- check-codex-skills
cargo run -- check-claude-skills
cargo run -- check-install
cargo run -- pre-commit
```

Config-internal preparation and home install behavior must also be verified without touching the real home directory:

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
- `~/.local/bin/codex` is a managed launcher that repairs deprecated Codex config flags before delegating to `/opt/homebrew/bin/codex`.
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
