use crate::cli::parse_config_args;
use crate::error::Result;
use crate::install::check_codex_skills_command;
use crate::managed_config::validate_managed_configs;
use crate::registry::validate_registry;
use crate::regression::run_regression_tests;
use std::path::Path;

pub(crate) fn validate_command(args: &[String]) -> Result<()> {
    let (config_root, _) = parse_config_args(args, false)?;
    let mut errors = validate_registry(&config_root);
    errors.extend(validate_managed_configs(&config_root));
    if !errors.is_empty() {
        let mut output = String::from("Config validation failed:");
        for error in errors {
            output.push_str("\n- ");
            output.push_str(&error);
        }
        return Err(output);
    }
    println!("Config validation passed.");
    Ok(())
}

pub(crate) fn test_validate_command(args: &[String]) -> Result<()> {
    if !args.is_empty() {
        return Err(format!("unknown option: {}", args[0]));
    }
    run_regression_tests()?;
    println!("Config regression tests passed.");
    Ok(())
}

pub(crate) fn check_command(args: &[String]) -> Result<()> {
    let (config_root, _) = parse_config_args(args, false)?;
    run_cargo(&config_root, &["fmt", "--check"])?;
    run_cargo(&config_root, &["check"])?;
    run_cargo(&config_root, &["test"])?;
    validate_command(&[
        "--config-root".to_string(),
        config_root.display().to_string(),
    ])?;
    test_validate_command(&[])?;
    Ok(())
}

pub(crate) fn prepare_command(args: &[String]) -> Result<()> {
    let (config_root, _) = parse_config_args(args, false)?;
    check_command(&[
        "--config-root".to_string(),
        config_root.display().to_string(),
    ])?;
    Ok(())
}

pub(crate) fn pre_commit_command(args: &[String]) -> Result<()> {
    let (config_root, _) = parse_config_args(args, false)?;
    prepare_command(&[
        "--config-root".to_string(),
        config_root.display().to_string(),
    ])?;
    check_codex_skills_command(&[
        "--config-root".to_string(),
        config_root.display().to_string(),
    ])
}

pub(crate) fn install_git_hooks_command(args: &[String]) -> Result<()> {
    let (config_root, _) = parse_config_args(args, false)?;
    let hooks_dir = config_root.join(".githooks");
    let hook_path = hooks_dir.join("pre-commit");
    if !hook_path.is_file() {
        return Err(format!(
            "{}: tracked pre-commit hook is missing",
            hook_path.display()
        ));
    }
    run_git(&config_root, &["config", "core.hooksPath", ".githooks"])?;
    println!(
        "Configured core.hooksPath=.githooks for {}",
        config_root.display()
    );
    Ok(())
}

fn run_cargo(config_root: &Path, args: &[&str]) -> Result<()> {
    run_command(config_root, "cargo", args)
}

fn run_git(config_root: &Path, args: &[&str]) -> Result<()> {
    run_command(config_root, "git", args)
}

fn run_command(config_root: &Path, program: &str, args: &[&str]) -> Result<()> {
    let status = std::process::Command::new(program)
        .args(args)
        .current_dir(config_root)
        .status()
        .map_err(|err| format!("cannot run {program} {}: {err}", args.join(" ")))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed with {status}", args.join(" ")))
    }
}
