//! Repository selection and the public reset contract. No config-root dependency.
pub(crate) mod cleanup;
mod discover;
mod git;
mod lock;
mod operation;
mod output;
mod status;
mod supervisor;

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
    pub json: bool,
    pub selection: serde_json::Value,
}

/// Clap owns syntax, help, version, suggestions, and typed value validation.
#[derive(Debug, clap::Args)]
pub(crate) struct ResetArgs {
    /// Repository or container directories (default: current directory)
    #[arg(value_name = "PATH", value_hint = clap::ValueHint::DirPath)]
    pub paths: Vec<PathBuf>,
    #[command(flatten)]
    pub scope: crate::workspace::ScopeArgs,
    /// Retry only unsuccessful targets from this run after fresh discovery and identity checks
    #[arg(long, conflicts_with_all=["paths","scope","list","remote","branch"])]
    pub retry: Option<String>,
    /// Remote to fetch
    #[arg(long, default_value = "origin", value_name = "NAME", value_parser = git_name)]
    pub remote: String,
    /// Branch to reset to (default: remote's advertised HEAD)
    #[arg(long, value_name = "NAME", value_parser = git_name)]
    pub branch: Option<String>,
    /// Concurrent repositories (1-32)
    #[arg(short = 'j', long, default_value_t = 32, value_name = "N", value_parser = clap::value_parser!(u32).range(1..=32))]
    pub jobs: u32,
    /// Total attempts per repository (1-10)
    #[arg(long, default_value_t = 3, value_name = "N", value_parser = clap::value_parser!(u32).range(1..=10))]
    pub attempts: u32,
    /// Deadline for the entire attempt (1-86400 seconds)
    #[arg(long, default_value_t = 300, value_name = "SECONDS", value_parser = clap::value_parser!(u32).range(1..=86_400))]
    pub timeout: u32,
    /// Discover and list targets without modifying repositories
    #[arg(long)]
    pub list: bool,
    /// Print complete logs on stderr in discovery order
    #[arg(short, long)]
    pub verbose: bool,
    /// Retain successful-run diagnostics in the system temporary directory
    #[arg(long)]
    pub keep_logs: bool,
}

fn git_name(value: &str) -> Result<String, String> {
    if value.is_empty() || value.starts_with('-') {
        Err("expected a nonempty Git name that does not begin with '-'".into())
    } else {
        Ok(value.into())
    }
}

pub(crate) fn worker(args: &[OsString]) -> u8 {
    operation::worker(args)
}
pub(crate) fn run(mut cli: ResetArgs, json: bool) -> u8 {
    let resolve = (|| -> Result<_, String> {
        if let Some(id) = &cli.retry {
            let prior = crate::history::read(id)?;
            if prior["operation"] != "git.reset" {
                return Err("--retry requires a reset run".into());
            }
            let mut paths = Vec::new();
            for target in prior["targets"]
                .as_array()
                .ok_or("invalid history targets")?
                .iter()
                .filter(|t| crate::history::unsuccessful(t))
            {
                let path = crate::history::verify_identity(target)?;
                let expected = crate::history::path(target, "common_bytes")?;
                let (repo, linked) =
                    discover::repository(&path)?.ok_or("retry target is no longer a repository")?;
                if linked || repo.path != path || repo.common != expected {
                    return Err("retry target repository ownership changed".into());
                }
                paths.push(path);
            }
            if paths.is_empty() {
                return Err("run has no unsuccessful reset targets".into());
            }
            if prior["options"]["retry_options_available"] != true {
                return Err("run options could not be recorded for safe retry; select explicit paths and options".into());
            }
            cli.paths = paths;
            cli.remote = prior["options"]["remote"]
                .as_str()
                .ok_or("run has no reusable remote name")?
                .to_owned();
            cli.branch = prior["options"]["branch"].as_str().map(str::to_owned);
        }
        cli.scope.resolve(&cli.paths)
    })();
    let resolved = match resolve {
        Ok(scope) => scope,
        Err(e) => {
            report(&e);
            if json {
                let _ = crate::presentation::json(
                    &serde_json::json!({"schema_version":1,"operation":"git.reset","status":"failed","error":e}),
                );
            }
            return 1;
        }
    };
    if cli.scope.scope.is_some() {
        cli.paths = resolved.roots.clone();
    } else if cli.paths.is_empty() {
        cli.paths.push(PathBuf::from("."));
    }
    let options = Options {
        selection: resolved.evidence(),
        paths: cli.paths,
        remote: cli.remote,
        branch: cli.branch,
        jobs: cli.jobs as usize,
        attempts: cli.attempts as usize,
        timeout: Duration::from_secs(cli.timeout.into()),
        list: cli.list,
        verbose: cli.verbose,
        keep_logs: cli.keep_logs,
        json,
    };
    let _signals = match crate::runtime::Signals::install() {
        Ok(s) => s,
        Err(e) => {
            report(&e);
            return 1;
        }
    };
    let started = std::time::Instant::now();
    match discover::select(&options.paths, &resolved.exclusions).and_then(|mut selection| {
        selection.repos.retain(|r| !resolved.excluded(&r.path));
        selection.excluded.retain(|p| !resolved.excluded(p));
        selection.skipped.retain(|p, _| !resolved.excluded(p));
        discover::report_skipped(&selection.skipped)?;
        Ok(selection)
    }) {
        Ok(selection) if options.list => {
            let result = serde_json::json!({"schema_version":1,"operation":"git.reset.inspect",
                "scope":crate::presentation::paths(&options.paths),"checkouts":crate::presentation::paths(selection.repos.iter().map(|r| &r.path)),
                "excluded":crate::presentation::paths(&selection.excluded),"skipped":discover::skipped_json(&selection.skipped),"status":if crate::runtime::interrupted() {"interrupted"} else {"completed"}});
            if json {
                let written = crate::presentation::json(&result);
                return if written == 0 && crate::runtime::interrupted() {
                    130
                } else {
                    written
                };
            }
            for r in selection.repos {
                if writeln!(std::io::stdout(), "{}", display(r.path)).is_err() {
                    return 1;
                }
            }
            for p in selection.excluded {
                if writeln!(
                    std::io::stderr(),
                    "Excluded linked worktree: {}",
                    display(p)
                )
                .is_err()
                {
                    return 1;
                }
            }
            if crate::runtime::cancelled() {
                130
            } else {
                0
            }
        }
        Ok(selection) => supervisor::run(&options, selection, started),
        Err(e) => {
            report(&e);
            if json {
                let _ = crate::presentation::json(
                    &serde_json::json!({"schema_version":1,"operation":"git.reset","status":if crate::runtime::interrupted() {"interrupted"} else {"failed"},"error":e}),
                );
            }
            if crate::runtime::cancelled() {
                130
            } else {
                1
            }
        }
    }
}
pub(super) fn report(error: &str) {
    let _ = writeln!(std::io::stderr(), "workctl: {}", display(error));
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
