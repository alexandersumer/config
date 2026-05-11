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
    let args: Vec<String> = env::args().skip(1).collect();
    let Some((command, command_args)) = args.split_first() else {
        print_help();
        return Ok(());
    };

    match command.as_str() {
        "generate" => generate_command(command_args),
        "validate" => validate_command(command_args),
        "test-validate" => test_validate_command(command_args),
        "check" => check_command(command_args),
        "prepare" => prepare_command(command_args),
        "pre-commit" => pre_commit_command(command_args),
        "install-git-hooks" => install_git_hooks_command(command_args),
        "install" => install_command(command_args),
        "repair-config" => repair_config_command(command_args),
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
