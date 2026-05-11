use crate::commands::{
    check_command, generate_command, install_git_hooks_command, pre_commit_command,
    prepare_command, test_validate_command, validate_command,
};
use crate::config_root::discover_config_root;
use crate::error::Result;
use crate::install::install_command;
use crate::repair::repair_config_command;
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
        "repair-config" => {
            args.remove(0);
            repair_config_command(&args)
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
    "Usage: config-tools <command> [options]\n\nCommands:\n  generate [--config-root PATH] [--check]\n  validate [--config-root PATH]\n  test-validate\n  check [--config-root PATH]\n  prepare [--config-root PATH]\n  pre-commit [--config-root PATH]\n  install-git-hooks [--config-root PATH]\n  repair-config [--config-root PATH]\n  install [--config-root PATH] [--home PATH]\n"
}

pub(crate) fn parse_config_args(args: &[String], allow_check: bool) -> Result<(PathBuf, bool)> {
    let mut config_root: Option<PathBuf> = None;
    let mut check = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--config-root" => {
                index += 1;
                let value = args.get(index).ok_or("--config-root requires a path")?;
                config_root = Some(PathBuf::from(value));
            }
            "--check" if allow_check => check = true,
            "--help" | "-h" => return Err(help_text().to_string()),
            unknown => return Err(format!("unknown option: {unknown}")),
        }
        index += 1;
    }

    let root = match config_root {
        Some(path) => path,
        None => discover_config_root(
            &env::current_exe()
                .map_err(|err| format!("cannot determine current executable: {err}"))?,
        )?,
    };
    root.canonicalize()
        .map(|root| (root, check))
        .map_err(|err| format!("{}: cannot resolve config root: {err}", root.display()))
}
