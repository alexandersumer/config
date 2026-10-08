//! Repository selection and the public reset contract. No config-root dependency.
mod discover;
mod git;
mod operation;
mod output;
mod status;
mod supervisor;

use clap::Parser;
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug)]
pub(super) struct Options {
    pub paths: Vec<PathBuf>,
    pub remote: String,
    pub branch: Option<String>,
    pub jobs: usize,
    pub attempts: usize,
    pub timeout: Duration,
    pub list: bool,
    pub verbose: bool,
    pub keep_logs: bool,
}

/// Clap owns syntax, help, version, suggestions, and typed value validation.
#[derive(Debug, Parser)]
#[command(
    name = "reset_to_origin",
    version,
    about = "Reset primary Git checkouts to a fetched remote branch",
    args_override_self = true,
    after_long_help = "Repository paths select one checkout; containers recursively select checkouts.\nDiscovery skips linked worktrees and does not descend into repositories or directory symlinks.\nExplicit linked worktrees are refused. Fetch configuration is respected, never repaired.\n\nDirty or colliding files cause refusal. Local branches and linked worktree files are preserved.\nReplaced branch tips are backed up. There is no rollback across repositories; run while\nrepositories are otherwise idle.\n\nExamples:\n  reset_to_origin\n  reset_to_origin --list .\n  reset_to_origin --jobs 4 /path/to/work /path/to/oss\n  reset_to_origin --remote upstream --branch release /path/to/repository\n\nExit codes: 0 success, 1 operational failure, 2 invalid usage, 130 interrupted."
)]
struct Cli {
    /// Repository or container directories (default: current directory)
    #[arg(value_name = "PATH", default_value = ".", value_hint = clap::ValueHint::DirPath)]
    paths: Vec<PathBuf>,
    /// Remote to fetch
    #[arg(long, default_value = "origin", value_name = "NAME", value_parser = git_name)]
    remote: String,
    /// Branch to reset to (default: remote's advertised HEAD)
    #[arg(long, value_name = "NAME", value_parser = git_name)]
    branch: Option<String>,
    /// Concurrent repositories (1-32)
    #[arg(short = 'j', long, default_value_t = 32, value_name = "N", value_parser = clap::value_parser!(u32).range(1..=32))]
    jobs: u32,
    /// Total attempts per repository (1-10)
    #[arg(long, default_value_t = 3, value_name = "N", value_parser = clap::value_parser!(u32).range(1..=10))]
    attempts: u32,
    /// Deadline for the entire attempt (1-86400 seconds)
    #[arg(long, default_value_t = 300, value_name = "SECONDS", value_parser = clap::value_parser!(u32).range(1..=86_400))]
    timeout: u32,
    /// Discover and list targets without modifying repositories
    #[arg(long)]
    list: bool,
    /// Print complete logs in discovery order
    #[arg(short, long)]
    verbose: bool,
    /// Retain successful-run diagnostics in the system temporary directory
    #[arg(long)]
    keep_logs: bool,
}

fn git_name(value: &str) -> Result<String, String> {
    if value.is_empty() || value.starts_with('-') {
        Err("expected a nonempty Git name that does not begin with '-'".into())
    } else {
        Ok(value.into())
    }
}

fn parse(args: Vec<OsString>) -> Result<Options, clap::Error> {
    let cli = Cli::try_parse_from(std::iter::once(OsString::from("reset_to_origin")).chain(args))?;
    Ok(Options {
        paths: cli.paths,
        remote: cli.remote,
        branch: cli.branch,
        jobs: cli.jobs as usize,
        attempts: cli.attempts as usize,
        timeout: Duration::from_secs(cli.timeout.into()),
        list: cli.list,
        verbose: cli.verbose,
        keep_logs: cli.keep_logs,
    })
}

// Keep Clap's generated layout while escaping controls in user-supplied values.
// Writing explicitly preserves our operational-failure contract for broken pipes.
fn print_cli_error(error: clap::Error) -> u8 {
    let rendered: String = error
        .render()
        .to_string()
        .chars()
        .flat_map(|c| {
            if c.is_control() && c != '\n' && c != '\t' {
                format!("\\u{{{:x}}}", c as u32).chars().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect();
    let result = if error.use_stderr() {
        std::io::stderr().write_all(rendered.as_bytes())
    } else {
        std::io::stdout().write_all(rendered.as_bytes())
    };
    if result.is_ok() {
        error.exit_code() as u8
    } else {
        1
    }
}

pub(crate) fn run(args: Vec<OsString>) -> u8 {
    // Workers are fresh executable processes in supervisor-owned process groups.
    if args.first().is_some_and(|a| a == "--internal-worker") {
        return operation::worker(&args[1..]);
    }
    let options = match parse(args) {
        Ok(options) => options,
        Err(error) => return print_cli_error(error),
    };
    let _signals = match supervisor::Signals::install() {
        Ok(signals) => signals,
        Err(error) => {
            report(&error);
            return 1;
        }
    };
    if !options.list {
        let mut stdout = std::io::stdout().lock();
        if let Err(error) =
            writeln!(stdout, "Discovering repositories...").and_then(|_| stdout.flush())
        {
            report(&error.to_string());
            return 1;
        }
    }
    match discover::select(&options.paths) {
        Ok(selection) => {
            if options.list {
                for repo in &selection.repos {
                    if supervisor::cancelled() {
                        return 130;
                    }
                    if writeln!(std::io::stdout(), "{}", display(&repo.path)).is_err() {
                        return 1;
                    }
                }
                for path in &selection.excluded {
                    if writeln!(
                        std::io::stderr(),
                        "Skipped linked worktree: {}",
                        display(path)
                    )
                    .is_err()
                    {
                        return 1;
                    }
                }
                return if supervisor::cancelled() { 130 } else { 0 };
            }
            supervisor::run(&options, selection)
        }
        Err(error) => {
            report(&error);
            if supervisor::cancelled() {
                130
            } else {
                1
            }
        }
    }
}

pub(super) fn report(error: &str) {
    let _ = writeln!(std::io::stderr(), "reset_to_origin: {}", display(error));
}

pub(super) fn display(value: impl AsRef<std::ffi::OsStr>) -> String {
    value
        .as_ref()
        .to_string_lossy()
        .chars()
        .flat_map(|c| {
            if c.is_control() {
                format!("\\u{{{:x}}}", c as u32).chars().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clap_command_definition_is_consistent() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn paths_are_not_remotes_and_legacy_options_fail() {
        let opts = parse(vec!["origin".into(), "--".into(), "-folder".into()]).unwrap();
        assert_eq!(
            opts.paths,
            vec![PathBuf::from("origin"), PathBuf::from("-folder")]
        );
        for flag in [
            "--single",
            "--multi",
            "--sync",
            "--no-prune",
            "--root",
            "--retries",
        ] {
            assert!(parse(vec![flag.into()]).is_err());
        }
        assert_eq!(parse(vec![]).unwrap().paths, vec![PathBuf::from(".")]);
    }
}
