# config

Personal configuration and agent-skill registry.

## Tooling

Repo tooling is implemented in Rust via the `config-tools` binary.

```bash
cargo run -- check
cargo run -- generate --check
cargo run -- validate
cargo run -- test-validate
cargo run -- repair-repo
cargo run -- install
```

## Git hooks

This repo uses Git's native tracked-hooks convention: `.githooks/pre-commit` is tracked, and the local checkout is configured with:

```bash
git config core.hooksPath .githooks
```

Use the Rust helper to configure that for this checkout:

```bash
cargo run -- install-git-hooks
```

The pre-commit hook delegates to Rust:

```bash
cargo run -- check
```

Git intentionally does not auto-enable arbitrary hooks from a freshly cloned repository for security reasons. A fresh checkout therefore needs one local setup step (`cargo run -- install-git-hooks`) before hooks will run. After that, hooks are enabled for that checkout.

## Acceptance criteria

A change is accepted only if all of these pass locally:

```bash
cargo fmt --check
cargo check
cargo run -- check
```

Repo-internal symlink repair and home install behavior must also be verified without touching the real home directory:

```bash
cargo run -- repair-repo

tmp_home="$(mktemp -d)"
cargo run -- install --home "$tmp_home"
find "$tmp_home" -maxdepth 2 -type l -exec ls -la {} \;
rm -rf "$tmp_home"
```

Expected symlink behavior:

- `rovodev/prompts` remains a symlink to `../.agents/skills`.
- `~/.agents` links to this repo's `.agents` directory.
- `~/.rovodev/skills` links to this repo's `.agents/skills` directory.
- `~/.rovodev/prompts` links to this repo's `.agents/skills` directory for legacy prompt compatibility.
- `~/.rovodev/prompts.yml` links to this repo's `rovodev/prompts.yml`.
- Existing non-empty files/directories or unrelated symlinks are not replaced automatically.

## Executable-code check

The only tracked non-Rust executable should be `.githooks/pre-commit`, because Git hooks must be executable files. It delegates to Rust and should contain no repo logic beyond `cargo run -- check`.

Verify with:

```bash
find . -path './.git' -prune -o -path './target' -prune -o -type f -perm -111 -print | sort
git grep -n -E '^#!|python3|/usr/bin/env|cargo run --quiet --manifest-path' -- ':!README.md'
```

The executable-file check should print only `./.githooks/pre-commit`.
