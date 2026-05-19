# config

Personal configuration and agent-skill registry.

## Tooling

Config tooling is implemented in Rust via the `config-tools` binary.

```bash
cargo run -- check
cargo run -- prepare
cargo run -- pre-commit
cargo run -- generate --check
cargo run -- validate
cargo run -- test-validate
cargo run -- repair-config
cargo run -- install
```

Command roles:

- `check`: non-mutating verification for formatting, build, generated registry, validation, and regression tests.
- `prepare`: intentional config-local mutation; repairs `rovodev/prompts`, regenerates `rovodev/prompts.yml`, then checks.
- `pre-commit`: safe hook entrypoint; runs `prepare`, then fails if config-local generated files are left unstaged.
- `install`: intentional home-directory mutation for `~/.agents`, `~/.rovodev`, and custom `~/.codex/skills` symlinks.
- `install-git-hooks`: intentional local Git config mutation for `core.hooksPath`.

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
cargo run -- pre-commit
```

Config-internal preparation and home install behavior must also be verified without touching the real home directory:

```bash
cargo run -- prepare

tmp_home="$(mktemp -d)"
for skill in skill-creator skill-installer openai-docs; do
  mkdir -p "$tmp_home/.codex/skills/.system/$skill"
  printf 'fixture\n' > "$tmp_home/.codex/skills/.system/$skill/SKILL.md"
done
cargo run -- install --home "$tmp_home"
find "$tmp_home" -maxdepth 4 -type l -exec ls -la {} \;
rm -rf "$tmp_home"
```

Expected symlink behavior:

- `rovodev/prompts` remains a symlink to `../.agents/skills`.
- `~/.agents` links to this config checkout's `.agents` directory.
- `~/.rovodev/skills` links to this config checkout's `.agents/skills` directory.
- `~/.rovodev/prompts` links to this config checkout's `.agents/skills` directory for legacy prompt compatibility.
- `~/.rovodev/prompts.yml` links to this config checkout's `rovodev/prompts.yml`.
- `~/.codex/skills/.system` remains a Codex-owned directory with Codex system skills.
- Each custom top-level `.agents/skills/<name>/SKILL.md` directory links into `~/.codex/skills/<name>`.
- Hidden skill directories and directories without `SKILL.md` are not linked into Codex.
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
./.agents/skills/brainstorming/scripts/start-server.sh
./.agents/skills/brainstorming/scripts/stop-server.sh
./.githooks/pre-commit
```

`.githooks/pre-commit` delegates to Rust and should contain no config-tool logic beyond `cargo run -- pre-commit`.
