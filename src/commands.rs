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
    // Registry validation needs no build, so invalid skills fail before compilation.
    validate_command(&[
        "--config-root".to_string(),
        config_root.display().to_string(),
    ])?;
    run_cargo(&config_root, &["fmt", "--check"])?;
    run_cargo(&config_root, &["check"])?;
    crate::validation::test_parallel(&config_root, &[])?;
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

    // A stable snapshot path lets its own Cargo target keep registry dependency
    // outputs between hooks. Its binaries still resolve the snapshot as their
    // config root, and Git discovery inside it stops at the cache directory.
    let cache = config_root.join("target/pre-commit");
    let snapshot = cache.join("snapshot");
    std::fs::create_dir_all(&snapshot)
        .map_err(|err| format!("{}: cannot create snapshot: {err}", snapshot.display()))?;
    let _lock = lock_snapshot_cache(&cache)?;
    reset_snapshot_source(&snapshot)?;
    let prefix = format!("--prefix={}/", snapshot.display());
    run_git(&config_root, &["checkout-index", "--all", &prefix])?;
    let ceiling = match std::env::var_os("GIT_CEILING_DIRECTORIES") {
        Some(existing) if !existing.is_empty() => {
            let mut value = cache.clone().into_os_string();
            value.push(":");
            value.push(existing);
            value
        }
        _ => cache.clone().into_os_string(),
    };

    // Git's hook environment must not redirect fixture commands to the real index.
    let git_vars = std::process::Command::new("git")
        .args(["rev-parse", "--local-env-vars"])
        .current_dir(&config_root)
        .output()
        .map_err(|err| format!("cannot inspect Git environment: {err}"))?;
    if !git_vars.status.success() {
        return Err("cannot inspect Git environment".to_string());
    }
    let git_vars: Vec<String> = String::from_utf8_lossy(&git_vars.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    let snapshot_cargo = |args: &[&str]| {
        let mut command = std::process::Command::new("cargo");
        command
            .args(args)
            .current_dir(&snapshot)
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("CARGO_BUILD_TARGET_DIR")
            .env("GIT_CEILING_DIRECTORIES", &ceiling);
        for name in &git_vars {
            command.env_remove(name);
        }
        command
    };

    // Local package outputs are discarded first, as in hosted caching, so staged
    // source is always rebuilt regardless of file timestamps.
    discard_local_package_outputs(&snapshot_cargo)?;
    let status = snapshot_cargo(&["run", "--", "prepare"])
        .status()
        .map_err(|err| format!("cannot validate staged snapshot: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("staged checks failed with {status}"))
    }
}

/// Serializes hooks that share a checkout's snapshot. Concurrent runs would
/// otherwise replace each other's source and test executables.
fn lock_snapshot_cache(cache: &Path) -> Result<std::fs::File> {
    use std::os::fd::AsRawFd;
    let path = cache.join(".lock");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|err| format!("{}: cannot open snapshot lock: {err}", path.display()))?;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Ok(file);
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() != Some(libc::EWOULDBLOCK) {
        return Err(format!("{}: cannot lock snapshot: {error}", path.display()));
    }
    eprintln!("Waiting for another pre-commit run in this checkout to finish...");
    loop {
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0 {
            return Ok(file);
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::Interrupted {
            return Err(format!("{}: cannot lock snapshot: {error}", path.display()));
        }
    }
}

/// Removes the previous snapshot's source and logs, keeping only its Cargo target.
fn reset_snapshot_source(snapshot: &Path) -> Result<()> {
    let entries = std::fs::read_dir(snapshot)
        .map_err(|err| format!("{}: cannot read snapshot: {err}", snapshot.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|err| format!("{}: cannot read snapshot: {err}", snapshot.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("{}: cannot inspect snapshot entry: {err}", path.display()))?;
        let removed = if entry.file_name() == "target" && file_type.is_dir() {
            let logs = path.join("validation-logs");
            match std::fs::remove_dir_all(&logs) {
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
                other => other,
            }
        } else if file_type.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        removed.map_err(|err| format!("{}: cannot reset snapshot: {err}", path.display()))?;
    }
    Ok(())
}

/// Keeps only immutable registry dependency outputs in the snapshot target.
fn discard_local_package_outputs(
    snapshot_cargo: &dyn Fn(&[&str]) -> std::process::Command,
) -> Result<()> {
    let output = snapshot_cargo(&["metadata", "--format-version", "1"])
        .output()
        .map_err(|err| format!("cannot read snapshot Cargo metadata: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot read snapshot Cargo metadata: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("invalid snapshot Cargo metadata: {err}"))?;
    let packages = metadata["packages"]
        .as_array()
        .filter(|packages| !packages.is_empty())
        .ok_or("snapshot Cargo metadata has no packages")?;
    let mut args = vec!["clean", "--quiet"];
    for package in packages {
        let name = package["name"]
            .as_str()
            .filter(|name| !name.is_empty())
            .ok_or("incomplete snapshot Cargo package metadata")?;
        let registry = match &package["source"] {
            serde_json::Value::Null => false,
            serde_json::Value::String(source) => source.starts_with("registry+"),
            _ => return Err("invalid snapshot Cargo package source".to_string()),
        };
        if !registry {
            args.extend(["--package", name]);
        }
    }
    if args.len() == 2 {
        return Err("snapshot Cargo metadata has no local package".to_string());
    }
    let status = snapshot_cargo(&args)
        .status()
        .map_err(|err| format!("cannot discard local package outputs: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "cannot discard local package outputs: cargo clean failed with {status}"
        ))
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
