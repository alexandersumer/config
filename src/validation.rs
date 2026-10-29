//! Build once with Cargo, then overlap independent test executables. Never cache results.
use crate::error::Result;
use std::io::{BufRead, BufReader, Read, Seek};
use std::path::Path;
use std::process::{Command, Stdio};

pub(crate) fn test_parallel(root: &Path, args: &[String]) -> Result<()> {
    // Keep the supported Cargo selection semantics explicit. In particular, arbitrary
    // harness filters must never accidentally turn a full check into a partial one.
    let mut index = 0;
    let mut selected = false;
    while index < args.len() {
        match args[index].as_str() {
            "--locked" => (),
            "--lib" => selected = true,
            "--test" => {
                selected = true;
                index += 1;
                if args.get(index).is_none_or(|name| name.starts_with('-')) {
                    return Err("--test requires a target name".into());
                }
            }
            other => return Err(format!("unsupported test-parallel option: {other}")),
        }
        index += 1;
    }
    let mut build = Command::new("cargo")
        .args([
            "test",
            "--no-run",
            "--message-format=json-render-diagnostics",
        ])
        .args(args)
        .current_dir(root)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot build cargo tests: {e}"))?;
    let mut executables = std::collections::BTreeMap::new();
    let mut parse_error = None;
    for line in BufReader::new(build.stdout.take().unwrap()).lines() {
        let parsed = line.map_err(|e| e.to_string()).and_then(|line| {
            serde_json::from_str::<serde_json::Value>(&line).map_err(|e| e.to_string())
        });
        match parsed {
            Ok(value) => {
                if value["reason"] == "compiler-artifact" && value["profile"]["test"] == true {
                    if let Some(exe) = value["executable"].as_str() {
                        let name = value["target"]["name"].as_str().unwrap_or("").to_owned();
                        executables.insert(exe.to_owned(), name);
                    }
                }
            }
            Err(error) => parse_error = Some(error),
        }
    }
    let status = build.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("cargo test build failed with {status}"));
    }
    if let Some(error) = parse_error {
        return Err(format!("cannot read cargo test inventory: {error}"));
    }
    if executables.is_empty() {
        return Err("cargo test produced no test executables".into());
    }
    let mut commands = Vec::new();
    let mut auth = Vec::new();
    let mut snapshots = Vec::new();
    for (exe, name) in executables {
        let mut command = Command::new(&exe);
        // Each suite contains subprocess-heavy fixtures. Bound per-suite workers
        // instead of multiplying the machine-wide default across all suites.
        command
            .args(["--nocapture", "--test-threads=2"])
            .current_dir(root);
        // Auth asserts prompt latency. Snapshot tests recursively run the entire
        // check. Neither may compete with the subprocess-heavy Git suites.
        match name.as_str() {
            "auth_cli" => auth.push((exe, command)),
            "pre_commit" => snapshots.push((exe, command)),
            _ => commands.push((exe, command)),
        }
    }
    // `cargo test --no-run` does not include doctests. Full checks still run them.
    if !selected {
        let mut command = Command::new("cargo");
        command.args(["test", "--doc"]).args(args).current_dir(root);
        commands.push(("doctests".into(), command));
    }
    let mut errors = Vec::new();
    for phase in [auth, snapshots, commands] {
        if !phase.is_empty() {
            if let Err(error) = run_all(root, phase) {
                errors.push(error);
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn run_all(root: &Path, commands: Vec<(String, Command)>) -> Result<()> {
    let log_root = std::env::var_os("RESET_E2E_ARTIFACT_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("target/validation-logs"));
    std::fs::create_dir_all(&log_root)
        .map_err(|e| format!("cannot create test log directory: {e}"))?;
    let logs = tempfile::Builder::new()
        .prefix("suites-")
        .tempdir_in(&log_root)
        .map_err(|e| format!("cannot create test logs: {e}"))?;
    println!("Suite logs: {}", logs.path().display());
    // Files avoid pipe deadlocks and unbounded in-memory output. Every started
    // suite is joined, even when another fails; logs are replayed without interleaving.
    let mut running = Vec::new();
    let mut errors = Vec::new();
    for (index, (name, mut command)) in commands.into_iter().enumerate() {
        println!("Starting {name}");
        let launch = (|| {
            let log = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(logs.path().join(format!("suite-{index}.log")))
                .map_err(|e| e.to_string())?;
            command.stdout(log.try_clone().map_err(|e| e.to_string())?);
            command.stderr(log.try_clone().map_err(|e| e.to_string())?);
            let child = command.spawn().map_err(|e| e.to_string())?;
            Ok::<_, String>((child, log))
        })();
        match launch {
            Ok((child, log)) => running.push((name, child, log)),
            Err(error) => errors.push(format!("{name}: {error}")),
        }
    }
    for (name, mut child, mut log) in running {
        let status = child.wait();
        println!("\n=== {name} ===");
        let replay = (|| {
            log.rewind()?;
            let mut buffer = [0; 8192];
            use std::io::Write;
            loop {
                let n = log.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                std::io::stdout().write_all(&buffer[..n])?;
            }
            Ok::<_, std::io::Error>(())
        })();
        if let Err(error) = replay {
            errors.push(format!("{name}: cannot read log: {error}"));
        }
        match status {
            Ok(status) if status.success() => (),
            Ok(status) => errors.push(format!("{name}: {status}")),
            Err(error) => errors.push(format!("{name}: {error}")),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        eprintln!("Preserved suite logs: {}", logs.keep().display());
        Err(format!("cargo test failed:\n{}", errors.join("\n")))
    }
}
