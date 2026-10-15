# Development and verification

[Back to README](../README.md)

Enable the tracked Git hook once per checkout. Repository checks use an isolated staged snapshot; installation checks are separate.

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

## Architecture and CLI verification

`src/workctl.rs` owns the public command hierarchy. `src/runtime.rs` owns process
capture, deadlines, cancellation, and group lifetime. `src/auth/` owns validated
credential profiles, provider adapters, bounded secret captures, and auth coordination. `src/presentation.rs` owns shared
rendering and JSON conventions. `src/git_domain/` owns topology, common-directory locks,
publication, and separate reset/cleanup policies. No plugin registry or command DSL
is involved.

```sh
cargo test --locked --lib --test reset_cli --test reset_e2e --test workctl --test auth_cli
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
