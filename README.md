# config

Personal configuration and agent-skill registry.

## Tooling

All executable repo tooling is implemented in Rust via the `config-tools` binary.

```bash
cargo run -- generate --check
cargo run -- validate
cargo run -- test-validate
cargo run -- repair-repo
cargo run -- install
```

## Acceptance criteria

A change is accepted only if all of these pass locally:

```bash
cargo fmt --check
cargo check
cargo run -- generate --check
cargo run -- validate
cargo run -- test-validate
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

## Pure Rust executable-code check

The repo should not contain tracked shell or Python executable tooling. Verify with:

```bash
git ls-files -s | awk 'substr($1,4,3) == "755" {print}'
git grep -n -E '^#!|python3|/usr/bin/env|cargo run --quiet --manifest-path' -- ':!README.md'
```

The executable-file check should print nothing. The grep check may still find shell configuration content under `zsh/`, but it should not find repo tooling scripts.
