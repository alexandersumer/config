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
    let mut executables = std::collections::BTreeSet::new();
    let mut parse_error = None;
    for line in BufReader::new(build.stdout.take().unwrap()).lines() {
        let parsed = line.map_err(|e| e.to_string()).and_then(|line| {
            serde_json::from_str::<serde_json::Value>(&line).map_err(|e| e.to_string())
        });
        match parsed {
            Ok(value) => {
                if value["reason"] == "compiler-artifact" && value["profile"]["test"] == true {
                    if let Some(exe) = value["executable"].as_str() {
                        executables.insert(exe.to_owned());
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
    for exe in executables {
        let mut command = Command::new(&exe);
        command.arg("--nocapture").current_dir(root);
        commands.push((exe, command));
    }
    // `cargo test --no-run` does not include doctests. Full checks still run them.
    if !selected {
        let mut command = Command::new("cargo");
        command.args(["test", "--doc"]).args(args).current_dir(root);
        commands.push(("doctests".into(), command));
    }
    run_all(commands)
}

fn run_all(commands: Vec<(String, Command)>) -> Result<()> {
    // Files avoid pipe deadlocks and unbounded in-memory output. Every started
    // suite is joined, even when another fails; logs are replayed without interleaving.
    let mut running = Vec::new();
    let mut errors = Vec::new();
    for (name, mut command) in commands {
        println!("Starting {name}");
        let launch = (|| {
            let log = tempfile::tempfile().map_err(|e| e.to_string())?;
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
        Err(format!("cargo test failed:\n{}", errors.join("\n")))
    }
}
