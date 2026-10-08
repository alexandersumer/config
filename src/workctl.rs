//! The public command hierarchy. Config installation remains an internal tool.
use clap::{CommandFactory, Parser, Subcommand};
use std::ffi::OsString;
use std::io::Write;

#[derive(Parser, Debug)]
#[command(
    name = "workctl",
    version,
    about = "Personal workstation maintenance",
    propagate_version = true,
    args_override_self = true,
    after_help = "Examples:\n  workctl git reset /path/to/work /path/to/oss\n  workctl git worktree clean /path/to/work\n  workctl doctor\n  workctl completions zsh\n\nExit codes: 0 success, 1 failed or blocked, 2 invalid usage, 130 interrupted."
)]
struct Cli {
    /// Emit a single schema-versioned JSON result on stdout
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Operation,
}
#[derive(Subcommand, Debug)]
enum Operation {
    /// Maintain Git checkouts and linked worktrees
    #[command(after_help = "Examples:\n  workctl git reset .\n  workctl git worktree clean .")]
    Git {
        #[command(subcommand)]
        command: Git,
    },
    /// Inspect installation, prerequisites, and supported capabilities without repairs
    #[command(after_help = "Examples:\n  workctl doctor\n  workctl doctor --json")]
    Doctor,
    /// Generate shell completion from the command hierarchy
    #[command(after_help = "Example:\n  workctl completions zsh > _workctl")]
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}
#[derive(Subcommand, Debug)]
enum Git {
    /// Reset primary checkouts to a verified, fetched remote branch
    #[command(
        args_override_self = true,
        after_long_help = "Discovery skips linked worktrees; explicit linked worktrees are refused.\nDirty or colliding files cause refusal. Other branches and noncolliding local files\nare preserved. Replaced target tips are backed up. There is no batch rollback.\nExit codes: 0 success, 1 failed or blocked, 2 invalid usage, 130 interrupted.\n\nExamples:\n  workctl git reset .\n  workctl git reset --list /path/to/work\n  workctl git reset --jobs 4 /path/to/work /path/to/oss\n  workctl git reset --remote upstream --branch release /path/to/repository"
    )]
    Reset(crate::git_domain::ResetArgs),
    /// Inspect and remove verified linked worktrees
    #[command(after_help = "Example:\n  workctl git worktree clean .")]
    Worktree {
        #[command(subcommand)]
        command: Worktree,
    },
}
#[derive(Subcommand, Debug)]
enum Worktree {
    /// Preview removal; --apply executes the plan without another question
    #[command(
        args_override_self = true,
        after_long_help = "Main checkouts are protected. Container scopes select linked worktrees beneath\nthem; a checkout selects its registered linked worktrees. Explicit linked paths\nselect only that worktree. Publication is checked against current remote heads\nand tags in a temporary object store, never cached remote-tracking references.\nLocks, missing paths, submodules, nested repositories, and uncertain ownership block\nremoval even with a discard decision. Run while repositories are otherwise idle.\nExit codes: 0 success, 1 failed or blocked, 2 invalid usage, 130 interrupted.\n\nExamples:\n  workctl git worktree clean /path/to/work /path/to/oss\n  workctl git worktree clean --apply /path/to/work\n  workctl git worktree clean --apply --discard-local /path/to/work/feature /path/to/work"
    )]
    Clean(crate::git_domain::cleanup::Args),
}

pub(crate) fn completions(shell: clap_complete::Shell) -> Vec<u8> {
    let mut bytes = Vec::new();
    clap_complete::generate(shell, &mut Cli::command(), "workctl", &mut bytes);
    bytes
}
pub(crate) fn run(args: Vec<OsString>) -> u8 {
    let cli = match Cli::try_parse_from(std::iter::once(OsString::from("workctl")).chain(args)) {
        Ok(cli) => cli,
        Err(error) => {
            let rendered = crate::presentation::safe_text(&error.render().to_string());
            let result = if error.use_stderr() {
                std::io::stderr().write_all(rendered.as_bytes())
            } else {
                std::io::stdout().write_all(rendered.as_bytes())
            };
            return if result.is_ok() {
                error.exit_code() as u8
            } else {
                1
            };
        }
    };
    match cli.command {
        Operation::Git {
            command: Git::Reset(args),
        } => crate::git_domain::run(args, cli.json),
        Operation::Git {
            command: Git::Worktree {
                command: Worktree::Clean(args),
            },
        } => crate::git_domain::cleanup::run(args, cli.json),
        Operation::Doctor => doctor(cli.json),
        Operation::Completions { shell } => {
            let bytes = completions(shell);
            if cli.json {
                crate::presentation::json(
                    &serde_json::json!({"schema_version":1,"operation":"completions","shell":shell.to_string(),"script":String::from_utf8_lossy(&bytes),"status":"completed"}),
                )
            } else {
                u8::from(std::io::stdout().write_all(&bytes).is_err())
            }
        }
    }
}
fn doctor(json: bool) -> u8 {
    let _signals = match crate::runtime::Signals::install() {
        Ok(s) => s,
        Err(e) => {
            let _ = writeln!(
                std::io::stderr(),
                "workctl: {}",
                crate::presentation::safe_text(&e.to_string())
            );
            return 1;
        }
    };
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            let _ = writeln!(
                std::io::stderr(),
                "workctl: {}",
                crate::presentation::safe_text(&e.to_string())
            );
            return 1;
        }
    };
    let mut git = std::process::Command::new("git");
    git.arg("--version")
        .current_dir(cwd)
        .stdin(std::process::Stdio::null());
    let version = crate::runtime::capture(git, std::time::Duration::from_secs(15), true);
    let temp = tempfile::tempdir();
    let supported = version.as_ref().is_ok_and(|(code, bytes, _)| {
        *code == 0
            && String::from_utf8_lossy(bytes)
                .split_whitespace()
                .nth(2)
                .is_some_and(|v| {
                    let n: Vec<_> = v.split('.').filter_map(|s| s.parse::<u32>().ok()).collect();
                    n.len() >= 2 && (n[0] > 2 || n[0] == 2 && n[1] >= 36)
                })
    });
    let executable = std::env::current_exe().ok();
    let locking = tempfile::tempfile().is_ok_and(|file| {
        use std::os::fd::AsRawFd;
        unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) == 0 }
    });
    // Successful spawn proves the OS accepted setpgid, not just that we compiled on Unix.
    let mut probe = std::process::Command::new("/bin/sh");
    probe.args(["-c", "exit 0"]);
    let groups = crate::runtime::capture(probe, std::time::Duration::from_secs(5), true)
        .is_ok_and(|(code, _, _)| code == 0);
    let ok = supported && temp.is_ok() && executable.is_some() && locking && groups;
    let checks = serde_json::json!({"executable":executable.as_ref().map(|p|p.to_string_lossy()),"git":version.as_ref().ok().map(|(_,o,_)|String::from_utf8_lossy(o).trim().to_owned()),"git_supported":supported,"temporary_directory":temp.is_ok(),"process_groups":groups,"repository_locks":locking});
    let code = if crate::runtime::cancelled() {
        130
    } else {
        u8::from(!ok)
    };
    let outcome = if code == 130 {
        "Interrupted"
    } else if code == 0 {
        "Completed"
    } else {
        "Failed"
    };
    if json { let write = crate::presentation::json(&serde_json::json!({"schema_version":1,"operation":"doctor","status":if code==130 {"interrupted"} else if code==0 {"completed"} else {"failed"},"checks":checks})); if write != 0 { return write; } }
    else if writeln!(std::io::stdout(), "Workstation doctor\n\nScope          Local installation\nExecutable     {}\nGit            {} (requires 2.36+)\nTemporary dir  {}\nProcess groups {}\nRepo locks     {}\n\n{}",
        executable.map(|p|crate::presentation::safe_text(&p.to_string_lossy())).unwrap_or("unavailable".into()),
        checks["git"].as_str().unwrap_or("unavailable"), if temp.is_ok() {"available"} else {"unavailable"}, if groups {"available"} else {"unavailable"}, if locking {"available"} else {"unavailable"}, crate::presentation::status(outcome,code!=0,false)).is_err() { return 1; }
    code
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hierarchy_is_consistent() {
        Cli::command().debug_assert();
    }
}
