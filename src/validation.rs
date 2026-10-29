//! Build once with Cargo, then overlap independent test executables. Never cache results.
use crate::error::Result;
use std::io::{BufRead, BufReader, Read, Seek, Write};
use std::path::Path;
use std::process::{Command, Stdio};

pub(crate) fn test_parallel(root: &Path, args: &[String]) -> Result<()> {
    // Keep the supported Cargo selection semantics explicit. In particular, arbitrary
    // harness filters must never accidentally turn a full check into a partial one.
    let mut index = 0;
    let mut selected = false;
    let mut worker_budget = 8;
    let mut budget_set = false;
    let mut cargo_args = Vec::new();
    while index < args.len() {
        let argument = &args[index];
        match args[index].as_str() {
            "--worker-budget" if !budget_set => {
                index += 1;
                worker_budget = args
                    .get(index)
                    .and_then(|value| value.parse::<usize>().ok())
                    .filter(|value| (2..=64).contains(value))
                    .ok_or("--worker-budget requires an integer from 2 through 64")?;
                budget_set = true;
            }
            "--locked" => cargo_args.push(argument),
            "--lib" => {
                selected = true;
                cargo_args.push(argument);
            }
            "--test" => {
                selected = true;
                cargo_args.push(argument);
                index += 1;
                if args.get(index).is_none_or(|name| name.starts_with('-')) {
                    return Err("--test requires a target name".into());
                }
                cargo_args.push(&args[index]);
            }
            other => return Err(format!("unsupported test-parallel option: {other}")),
        }
        index += 1;
    }
    let harness_limit = match std::env::var("RUST_TEST_THREADS") {
        Ok(value) => value
            .parse::<usize>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or("RUST_TEST_THREADS must be a positive integer")?,
        Err(std::env::VarError::NotPresent) => usize::MAX,
        Err(error) => return Err(format!("invalid RUST_TEST_THREADS: {error}")),
    };
    let mut build = Command::new("cargo")
        .args([
            "test",
            "--no-run",
            "--message-format=json-render-diagnostics",
        ])
        .args(&cargo_args)
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
                if value["reason"] == "compiler-artifact" {
                    let Some(is_test) = value["profile"]["test"].as_bool() else {
                        parse_error = Some("compiler artifact has no test profile".into());
                        continue;
                    };
                    if is_test {
                        let (Some(exe), Some(name), Some(kind)) = (
                            value["executable"].as_str(),
                            value["target"]["name"].as_str(),
                            value["target"]["kind"].as_array(),
                        ) else {
                            parse_error = Some("incomplete test artifact inventory".into());
                            continue;
                        };
                        if kind.is_empty() || kind.iter().any(|item| !item.is_string()) {
                            parse_error = Some("invalid test target kind".into());
                            continue;
                        }
                        let heavy_git = value["target"]["kind"] == serde_json::json!(["test"])
                            && matches!(name, "workctl" | "reset_cli");
                        executables.insert(exe.to_owned(), (name.to_owned(), heavy_git));
                    }
                } else if !matches!(
                    value["reason"].as_str(),
                    Some("compiler-message" | "build-script-executed" | "build-finished")
                ) {
                    parse_error = Some("unknown or missing Cargo message reason".into());
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
    let heavy_count = executables
        .values()
        .filter(|(_, heavy)| *heavy)
        .count()
        .clamp(1, 2);
    let mut commands = Vec::new();
    let mut heavy = Vec::new();
    let mut auth = Vec::new();
    let mut snapshots = Vec::new();
    let mut fixtures = Vec::new();
    for (exe, (name, heavy_git)) in executables {
        let mut command = Command::new(&exe);
        // Each suite contains subprocess-heavy fixtures. Bound per-suite workers
        // instead of multiplying the machine-wide default across all suites.
        let available = std::thread::available_parallelism().map_or(1, |n| n.get());
        let desired = if heavy_git {
            worker_budget / heavy_count
        } else {
            worker_budget
        };
        let workers = available.min(desired).min(harness_limit);
        let workers = if budget_set { workers } else { workers.min(4) };
        command
            .arg("--nocapture")
            .arg(format!("--test-threads={workers}"))
            .current_dir(root);
        // Auth asserts prompt latency. Snapshot tests recursively run the entire
        // check. Neither may compete with the subprocess-heavy Git suites.
        match name.as_str() {
            "auth_cli" => auth.push((exe, command)),
            "pre_commit" => snapshots.push((exe, command)),
            "validation" => fixtures.push((exe, command)),
            _ if heavy_git => heavy.push((exe, command)),
            _ => commands.push((exe, command)),
        }
    }
    // Start long Git suites first; short targets use a slot after one finishes.
    // Cargo-building fixtures and doctests wait until the Git suites are done.
    heavy.extend(commands);
    let commands = heavy;
    // `cargo test --no-run` does not include doctests. Full checks still run them.
    if !selected {
        let mut command = Command::new("cargo");
        command
            .args(["test", "--doc"])
            .args(&cargo_args)
            .current_dir(root);
        fixtures.push(("doctests".into(), command));
    }
    let mut errors = Vec::new();
    for phase in [auth, snapshots, commands, fixtures] {
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
    run_all_to(root, commands, &mut std::io::stdout())
}

fn run_all_to(
    root: &Path,
    commands: Vec<(String, Command)>,
    output: &mut impl Write,
) -> Result<()> {
    const MAX_PROCESSES: usize = 2;
    let log_root = std::env::var_os("RESET_E2E_ARTIFACT_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("target/validation-logs"));
    std::fs::create_dir_all(&log_root)
        .map_err(|e| format!("cannot create test log directory: {e}"))?;
    let logs = tempfile::Builder::new()
        .prefix("suites-")
        .tempdir_in(&log_root)
        .map_err(|e| format!("cannot create test logs: {e}"))?;
    writeln!(output, "Suite logs: {}", logs.path().display())
        .map_err(|error| format!("cannot report test progress: {error}"))?;
    // Files avoid pipe deadlocks and unbounded in-memory output. Every started
    // suite is joined, even when another fails; logs are replayed without interleaving.
    let mut running = Vec::new();
    let mut completed = Vec::new();
    let mut errors = Vec::new();
    let mut pending = commands.into_iter().enumerate();
    loop {
        while running.len() < MAX_PROCESSES {
            let Some((index, (name, mut command))) = pending.next() else {
                break;
            };
            if let Err(error) = writeln!(output, "Starting {name}") {
                errors.push(format!("{name}: cannot report test progress: {error}"));
            }
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
                Ok((child, log)) => running.push((index, name, child, log)),
                Err(error) => errors.push(format!("{name}: {error}")),
            }
        }
        if running.is_empty() {
            break;
        }
        let ready = running
            .iter_mut()
            .position(|(_, name, child, _)| match child.try_wait() {
                Ok(status) => status.is_some(),
                Err(error) => {
                    errors.push(format!("{name}: cannot inspect process: {error}"));
                    true
                }
            });
        if let Some(index) = ready {
            let (order, name, mut child, log) = running.swap_remove(index);
            completed.push((order, name, child.wait(), log));
        } else {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    completed.sort_by_key(|(order, _, _, _)| *order);
    for (_, name, status, mut log) in completed {
        if let Err(error) = writeln!(output, "\n=== {name} ===") {
            errors.push(format!("{name}: cannot report test progress: {error}"));
        }
        let replay = (|| {
            log.rewind()?;
            let mut buffer = [0; 8192];
            loop {
                let n = log.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                output.write_all(&buffer[..n])?;
            }
            Ok::<_, std::io::Error>(())
        })();
        if let Err(error) = replay {
            errors.push(format!("{name}: cannot replay log: {error}"));
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

#[cfg(all(test, unix))]
mod tests {
    use super::{run_all, run_all_to};
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::{Duration, Instant};

    struct Release(Vec<PathBuf>);
    impl Drop for Release {
        fn drop(&mut self) {
            for path in &self.0 {
                let _ = fs::write(path, "release");
            }
        }
    }

    fn wait_for(path: &std::path::Path) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !path.exists() {
            assert!(
                Instant::now() < deadline,
                "process did not start: {}",
                path.display()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn output_failure_still_starts_and_joins_every_selected_child() {
        use std::sync::atomic::{AtomicBool, Ordering};
        struct BrokenOutput<'a>(&'a AtomicBool);
        impl std::io::Write for BrokenOutput<'_> {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if self.0.load(Ordering::SeqCst) {
                    Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
                } else {
                    Ok(bytes.len())
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        let starts: Vec<_> = (0..3).map(|i| root.join(format!("started-{i}"))).collect();
        let releases: Vec<_> = (0..3).map(|i| root.join(format!("release-{i}"))).collect();
        let finishes: Vec<_> = (0..3).map(|i| root.join(format!("finished-{i}"))).collect();
        let commands = (0..3).map(|i| {
            let mut cmd = Command::new("sh");
            cmd.args(["-c", "touch \"$STARTED\"; while [ ! -e \"$RELEASE\" ]; do [ -d \"$ROOT\" ] || exit 74; sleep 0.05; done; touch \"$FINISHED\""])
                .env("STARTED", &starts[i]).env("RELEASE", &releases[i])
                .env("FINISHED", &finishes[i]).env("ROOT", root);
            (format!("fixture-{i}"), cmd)
        }).collect();
        let broken = AtomicBool::new(false);
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| run_all_to(root, commands, &mut BrokenOutput(&broken)));
            let _release = Release(releases.clone());
            wait_for(&starts[0]);
            wait_for(&starts[1]);
            broken.store(true, Ordering::SeqCst);
            fs::write(&releases[0], "release").unwrap();
            wait_for(&starts[2]);
            for release in &releases {
                fs::write(release, "release").unwrap();
            }
            let error = worker.join().unwrap().unwrap_err();
            assert!(error.contains("cannot report test progress"));
            assert!(
                finishes.iter().all(|file| file.exists()),
                "output failure must not leave children running"
            );
        });
    }

    #[test]
    fn test_process_budget_bounds_live_children_and_runs_the_queue() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        let starts: Vec<_> = (0..3).map(|i| root.join(format!("started-{i}"))).collect();
        let releases: Vec<_> = (0..3).map(|i| root.join(format!("release-{i}"))).collect();
        let commands = (0..3)
            .map(|i| {
                let mut cmd = Command::new("sh");
                cmd.args([
                    "-c",
                    "touch \"$STARTED\"; while [ ! -e \"$RELEASE\" ]; do sleep 0.05; done",
                ])
                .env("STARTED", &starts[i])
                .env("RELEASE", &releases[i]);
                (format!("fixture-{i}"), cmd)
            })
            .collect();
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| run_all(root, commands));
            // Release every child even if an assertion fails; the scoped worker
            // joins before the fixture is removed.
            let _release = Release(releases.clone());
            wait_for(&starts[0]);
            wait_for(&starts[1]);
            std::thread::sleep(Duration::from_millis(250));
            let third_started_early = starts[2].exists();
            fs::write(&releases[0], "release").unwrap();
            fs::write(&releases[1], "release").unwrap();
            wait_for(&starts[2]);
            fs::write(&releases[2], "release").unwrap();
            worker.join().unwrap().unwrap();
            assert!(
                !third_started_early,
                "more than two test processes overlapped"
            );
        });
    }
}
