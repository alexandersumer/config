# config

Personal workstation configuration, shared agent skills, and the `workctl` CLI.

## Set up

Run from this checkout with Rust and Cargo installed:

```sh
cargo run -- install
cargo run -- install-git-hooks
```

`install` links the managed shell, Ghostty, Relay, and agent configuration into
this user's home directory. It also installs `config-tools` and `workctl` in
`~/.local/bin`. Existing non-managed files are preserved. Use `--home PATH` to
install into an isolated home instead.

Open a fresh shell after installation. To verify the installed configuration:

```sh
cargo run -- check-install
```

See [Configuration and skills](docs/configuration.md) for managed paths,
individual checks, and skill updates.

## Everyday commands

`workctl` handles Git maintenance and authentication readiness. Use `--help`
after any command to see its options.

| Task | Command |
| --- | --- |
| Inspect reset targets without fetching | `workctl git reset --list ~/src` |
| Reset primary checkouts to their remote default branch | `workctl git reset ~/src` |
| Preview linked-worktree cleanup | `workctl git worktree clean ~/src` |
| Apply linked-worktree cleanup | `workctl git worktree clean --apply ~/src` |
| Renew and verify work credentials | `workctl auth preflight` |
| Check code credentials, then run a command | `workctl auth exec --profile code -- bb pr list` |
| Check CLI prerequisites | `workctl doctor` |

Replace `~/src` with a repository or workspace directory. With no path, Git
commands use the current directory.

Reset refuses tracked changes and other unsafe states. Cleanup previews by
default. With `--apply`, it protects active local work, saves committed history
in recovery refs, and removes eligible worktrees **including their ignored
files**. Review the preview before applying cleanup across a workspace.
See the [workctl guide](docs/workctl.md) for authentication, Git policies, and recovery commands.

## Use custom skills

Custom skills live in [`.agents/skills`](.agents/skills). Installation links the
same source directories into `~/.codex/skills` and `~/.claude/skills`.

In Codex, invoke a skill with a skill mention, for example:

```text
$surgical-edit
$review-solo
$git-publish-to-origin
```

For discovery checks and TWG refresh instructions, see
[Configuration and skills](docs/configuration.md#custom-skills).

## Change this repository

Enable the Git hook with `cargo run -- install-git-hooks`. Each commit then runs
`cargo run -- pre-commit` against an isolated snapshot of the staged files.
This checks formatting, compilation, tests, and skill validation. Installation
drift is checked separately and does not block commits.

See [Development and verification](docs/development.md) for check commands,
architecture, and reset benchmarks. For isolated installation verification, see
[Configuration and skills](docs/configuration.md#verify-installation).

## Reference

- [Configuration and skills](docs/configuration.md): installers, managed links, Codex and Claude Code skills, TWG updates.
- [workctl guide](docs/workctl.md): installation, commands, authentication, Git operations, output, and exit status.
- [Git reference](docs/git.md): reset and cleanup policies, recovery, coordination, and JSON fields.
- [Authentication readiness](docs/authentication.md): profiles, renewal, MFA, command execution, and custom configuration.
- [Development and verification](docs/development.md): hooks, acceptance checks, architecture, and CLI tests.
