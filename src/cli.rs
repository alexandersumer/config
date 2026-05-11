use crate::commands::{
    check_command, generate_command, install_git_hooks_command, pre_commit_command,
    prepare_command, test_validate_command, validate_command,
};
use crate::error::Result;
use crate::install::install_command;
use crate::repair::repair_repo_command;
use crate::repo::discover_repo_root;
use std::env;
use std::path::PathBuf;

pub(crate) fn run() -> Result<()> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("help");
    match command {
        "generate" => {
            args.remove(0);
            generate_command(&args)
        }
        "validate" => {
            args.remove(0);
            validate_command(&args)
        }
        "test-validate" => {
            args.remove(0);
            test_validate_command(&args)
        }
        "check" => {
            args.remove(0);
            check_command(&args)
        }
        "prepare" => {
            args.remove(0);
            prepare_command(&args)
        }
        "pre-commit" => {
            args.remove(0);
            pre_commit_command(&args)
        }
        "install-git-hooks" => {
            args.remove(0);
            install_git_hooks_command(&args)
        }
        "install" => {
            args.remove(0);
            install_command(&args)
        }
        "repair-repo" => {
            args.remove(0);
            repair_repo_command(&args)
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command: {other}\n\n{}", help_text())),
    }
}

fn print_help() {
    println!("{}", help_text());
}

fn help_text() -> &'static str {
    "Usage: config-tools <command> [options]\n\nCommands:\n  generate [--repo-root PATH] [--check]\n  validate [--repo-root PATH]\n  test-validate\n  check [--repo-root PATH]\n  prepare [--repo-root PATH]\n  pre-commit [--repo-root PATH]\n  install-git-hooks [--repo-root PATH]\n  repair-repo [--repo-root PATH]\n  install [--repo-root PATH] [--home PATH]\n"
}

pub(crate) fn parse_repo_args(args: &[String], allow_check: bool) -> Result<(PathBuf, bool)> {
    let mut repo_root: Option<PathBuf> = None;
    let mut check = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                let value = args.get(index).ok_or("--repo-root requires a path")?;
                repo_root = Some(PathBuf::from(value));
            }
            "--check" if allow_check => check = true,
            "--help" | "-h" => return Err(help_text().to_string()),
            unknown => return Err(format!("unknown option: {unknown}")),
        }
        index += 1;
    }

    let root = match repo_root {
        Some(path) => path,
        None => discover_repo_root(
            &env::current_exe()
                .map_err(|err| format!("cannot determine current executable: {err}"))?,
        )?,
    };
    root.canonicalize()
        .map(|root| (root, check))
        .map_err(|err| format!("{}: cannot resolve repository root: {err}", root.display()))
}
