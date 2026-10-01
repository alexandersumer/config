use crate::cli::parse_config_args;
use crate::error::Result;
use crate::registry::validate_registry;
use crate::regression::run_regression_tests;
use std::path::Path;

pub(crate) fn validate_command(args: &[String]) -> Result<()> {
    let (config_root, _) = parse_config_args(args, false)?;
    let errors = validate_registry(&config_root);
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
    if !config_root.join(".git").exists() {
        return prepare_command(args);
    }

    let snapshot = tempfile::Builder::new()
        .prefix("config-pre-commit-")
        .tempdir()
        .map_err(|err| format!("cannot create staged snapshot: {err}"))?;
    let prefix = format!("--prefix={}/", snapshot.path().display());
    run_git(&config_root, &["checkout-index", "--all", &prefix])?;

    // Git's hook environment must not redirect fixture commands to the real index.
    let git_vars = std::process::Command::new("git")
        .args(["rev-parse", "--local-env-vars"])
        .current_dir(&config_root)
        .output()
        .map_err(|err| format!("cannot inspect Git environment: {err}"))?;
    if !git_vars.status.success() {
        return Err("cannot inspect Git environment".to_string());
    }
    let mut command = std::process::Command::new("cargo");
    command
        .args(["run", "--target-dir"])
        .arg(snapshot.path().join("target"))
        .args(["--", "prepare"])
        .current_dir(snapshot.path())
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_BUILD_TARGET_DIR");
    for name in String::from_utf8_lossy(&git_vars.stdout).lines() {
        command.env_remove(name);
    }
    let status = command
        .status()
        .map_err(|err| format!("cannot validate staged snapshot: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("staged checks failed with {status}"))
    }
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
