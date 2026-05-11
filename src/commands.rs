use crate::cli::parse_repo_args;
use crate::error::Result;
use crate::registry::{render_registry, validate_registry};
use crate::regression::run_regression_tests;
use crate::repair::repair_repo_command;
use std::fs;
use std::path::Path;

pub(crate) fn generate_command(args: &[String]) -> Result<()> {
    let (repo_root, check) = parse_repo_args(args, true)?;
    let output_path = repo_root.join("rovodev/prompts.yml");
    let rendered = render_registry(&repo_root)?;

    if check {
        let current = fs::read_to_string(&output_path).map_err(|err| {
            format!(
                "Prompt generation check failed:\n- {}: cannot read file: {err}",
                output_path.display()
            )
        })?;
        if current != rendered {
            return Err(format!(
                "Prompt generation check failed:\n- {}: generated content is not up to date",
                output_path.display()
            ));
        }
        println!("Prompt registry is up to date.");
        return Ok(());
    }

    fs::write(&output_path, rendered)
        .map_err(|err| format!("{}: cannot write file: {err}", output_path.display()))?;
    println!("Generated {}", output_path.display());
    Ok(())
}

pub(crate) fn validate_command(args: &[String]) -> Result<()> {
    let (repo_root, _) = parse_repo_args(args, false)?;
    let errors = validate_registry(&repo_root);
    if !errors.is_empty() {
        let mut output = String::from("Skill validation failed:");
        for error in errors {
            output.push_str("\n- ");
            output.push_str(&error);
        }
        return Err(output);
    }
    println!("Skill validation passed.");
    Ok(())
}

pub(crate) fn test_validate_command(args: &[String]) -> Result<()> {
    if !args.is_empty() {
        return Err(format!("unknown option: {}", args[0]));
    }
    run_regression_tests()?;
    println!("Skill validator regression tests passed.");
    Ok(())
}

pub(crate) fn check_command(args: &[String]) -> Result<()> {
    let (repo_root, _) = parse_repo_args(args, false)?;
    run_cargo(&repo_root, &["fmt", "--check"])?;
    run_cargo(&repo_root, &["check"])?;
    generate_command(&[
        "--repo-root".to_string(),
        repo_root.display().to_string(),
        "--check".to_string(),
    ])?;
    validate_command(&["--repo-root".to_string(), repo_root.display().to_string()])?;
    test_validate_command(&[])?;
    Ok(())
}

pub(crate) fn prepare_command(args: &[String]) -> Result<()> {
    let (repo_root, _) = parse_repo_args(args, false)?;
    repair_repo_command(&["--repo-root".to_string(), repo_root.display().to_string()])?;
    generate_command(&["--repo-root".to_string(), repo_root.display().to_string()])?;
    check_command(&["--repo-root".to_string(), repo_root.display().to_string()])?;
    Ok(())
}

pub(crate) fn pre_commit_command(args: &[String]) -> Result<()> {
    let (repo_root, _) = parse_repo_args(args, false)?;
    let before = repo_generated_snapshot(&repo_root)?;
    prepare_command(&["--repo-root".to_string(), repo_root.display().to_string()])?;
    let after = repo_generated_snapshot(&repo_root)?;
    if before != after {
        return Err(
            "pre-commit updated repo-local generated files. Review and stage rovodev/prompts and rovodev/prompts.yml, then commit again."
                .to_string(),
        );
    }
    ensure_no_unstaged_repo_generated_changes(&repo_root)
}

fn repo_generated_snapshot(repo_root: &Path) -> Result<Vec<(String, String)>> {
    ["rovodev/prompts", "rovodev/prompts.yml"]
        .iter()
        .map(|path| {
            let full_path = repo_root.join(path);
            let metadata = fs::symlink_metadata(&full_path).map_err(|err| {
                format!(
                    "{}: cannot inspect generated path: {err}",
                    full_path.display()
                )
            })?;
            let value = if metadata.file_type().is_symlink() {
                format!(
                    "symlink:{}",
                    fs::read_link(&full_path)
                        .map_err(|err| format!(
                            "{}: cannot read symlink: {err}",
                            full_path.display()
                        ))?
                        .display()
                )
            } else if metadata.is_file() {
                format!(
                    "file:{}",
                    fs::read_to_string(&full_path).map_err(|err| format!(
                        "{}: cannot read file: {err}",
                        full_path.display()
                    ))?
                )
            } else if metadata.is_dir() {
                "directory".to_string()
            } else {
                "other".to_string()
            };
            Ok((path.to_string(), value))
        })
        .collect()
}

fn ensure_no_unstaged_repo_generated_changes(repo_root: &Path) -> Result<()> {
    let paths = ["rovodev/prompts", "rovodev/prompts.yml"];
    let status = std::process::Command::new("git")
        .arg("diff")
        .arg("--quiet")
        .arg("--")
        .args(paths)
        .current_dir(repo_root)
        .status()
        .map_err(|err| format!("cannot run git diff: {err}"))?;
    if status.success() {
        Ok(())
    } else if status.code() == Some(1) {
        Err(
            "pre-commit updated or found unstaged repo-local generated files. Review and stage rovodev/prompts and rovodev/prompts.yml, then commit again."
                .to_string(),
        )
    } else {
        Err(format!("git diff failed with {status}"))
    }
}

pub(crate) fn install_git_hooks_command(args: &[String]) -> Result<()> {
    let (repo_root, _) = parse_repo_args(args, false)?;
    let hooks_dir = repo_root.join(".githooks");
    let hook_path = hooks_dir.join("pre-commit");
    if !hook_path.is_file() {
        return Err(format!(
            "{}: tracked pre-commit hook is missing",
            hook_path.display()
        ));
    }
    run_git(&repo_root, &["config", "core.hooksPath", ".githooks"])?;
    println!(
        "Configured core.hooksPath=.githooks for {}",
        repo_root.display()
    );
    Ok(())
}

fn run_cargo(repo_root: &Path, args: &[&str]) -> Result<()> {
    run_command(repo_root, "cargo", args)
}

fn run_git(repo_root: &Path, args: &[&str]) -> Result<()> {
    run_command(repo_root, "git", args)
}

fn run_command(repo_root: &Path, program: &str, args: &[&str]) -> Result<()> {
    let status = std::process::Command::new(program)
        .args(args)
        .current_dir(repo_root)
        .status()
        .map_err(|err| format!("cannot run {program} {}: {err}", args.join(" ")))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed with {status}", args.join(" ")))
    }
}
