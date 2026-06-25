use crate::commands::{
    check_command, install_git_hooks_command, pre_commit_command, prepare_command,
    test_validate_command, validate_command,
};
use crate::config_root::default_config_root;
use crate::error::Result;
use crate::install::{
    check_claude_skills_command, check_codex_skills_command, check_install_command,
    install_command, repair_codex_config_command,
};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

pub(crate) fn run() -> Result<ExitCode> {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some((command, command_args)) = args.split_first() else {
        print_help();
        return Ok(ExitCode::SUCCESS);
    };

    match command.as_str() {
        "validate" => validate_command(command_args).map(|()| ExitCode::SUCCESS),
        "test-validate" => test_validate_command(command_args).map(|()| ExitCode::SUCCESS),
        "check" => check_command(command_args).map(|()| ExitCode::SUCCESS),
        "prepare" => prepare_command(command_args).map(|()| ExitCode::SUCCESS),
        "pre-commit" => pre_commit_command(command_args).map(|()| ExitCode::SUCCESS),
        "install-git-hooks" => install_git_hooks_command(command_args).map(|()| ExitCode::SUCCESS),
        "check-codex-skills" => {
            check_codex_skills_command(command_args).map(|()| ExitCode::SUCCESS)
        }
        "check-claude-skills" => {
            check_claude_skills_command(command_args).map(|()| ExitCode::SUCCESS)
        }
        "check-install" => check_install_command(command_args).map(|()| ExitCode::SUCCESS),
        "repair-codex-config" => {
            repair_codex_config_command(command_args).map(|()| ExitCode::SUCCESS)
        }
        "install" => install_command(command_args).map(|()| ExitCode::SUCCESS),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(ExitCode::SUCCESS)
        }
        other => Err(format!("unknown command: {other}\n\n{}", help_text())),
    }
}

fn print_help() {
    println!("{}", help_text());
}

fn help_text() -> &'static str {
    "Usage: config-tools <command> [options]\n\nCommands:\n  validate [--config-root PATH]\n  test-validate\n  check [--config-root PATH]\n  prepare [--config-root PATH]\n  pre-commit [--config-root PATH]\n  install-git-hooks [--config-root PATH]\n  check-codex-skills [--config-root PATH] [--home PATH]\n  check-claude-skills [--config-root PATH] [--home PATH]\n  check-install [--config-root PATH] [--home PATH]\n  repair-codex-config [--home PATH]\n  install [--config-root PATH] [--home PATH]\n"
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
        None => default_config_root()?,
    };
    root.canonicalize()
        .map(|root| (root, check))
        .map_err(|err| format!("{}: cannot resolve config root: {err}", root.display()))
}
