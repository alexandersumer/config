use std::fs;
use std::process::Command;

fn run(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_config-tools"))
        .arg("test-parallel")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap()
}

#[test]
fn parallel_validation_discovers_targets_joins_failures_and_runs_doctests() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    fs::create_dir(root.join("src")).unwrap();
    fs::create_dir(root.join("tests")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"runner-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "/// ```\n/// assert_eq!(1, 2);\n/// ```\npub fn value() {}\n",
    )
    .unwrap();
    for name in ["alpha", "beta"] {
        let other = if name == "alpha" { "beta" } else { "alpha" };
        fs::write(
            root.join(format!("tests/{name}.rs")),
            format!(
                r#"
#[test]
fn rendezvous() {{
    std::fs::write("{name}.started", "yes").unwrap();
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !std::path::Path::new("{other}.started").exists() {{
        assert!(std::time::Instant::now() < until, "suites did not overlap");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }}
    println!("{{}}", "diagnostic".repeat(20000));
    std::fs::write("{name}.finished", "yes").unwrap();
    assert!("{name}" != "alpha", "representative integration regression");
}}
"#
            ),
        )
        .unwrap();
    }
    let result = run(root, &[]);
    assert!(!result.status.success());
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(
        output.contains("representative integration regression"),
        "{output}"
    );
    assert!(output.contains("Doc-tests runner_fixture"), "{output}");
    assert!(output.contains("diagnosticdiagnostic"));
    assert!(root.join("alpha.finished").exists());
    assert!(root.join("beta.finished").exists());

    // Changing source after a successful build must not reuse a stale result or binary.
    fs::write(root.join("src/lib.rs"), "invalid Rust\n").unwrap();
    let result = run(root, &["--lib"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("cargo test build failed"));
    assert!(!run(root, &["--test", "absent"]).status.success());
    assert!(!run(root, &["--", "rendezvous"]).status.success());
    assert!(!run(root, &["--worker-budget", "0"]).status.success());
    assert!(!run(root, &["--worker-budget", "65"]).status.success());
    assert!(!run(root, &["--worker-budget", "invalid"]).status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("error:"));

    // Cargo must revalidate a shared dependency and a stale lockfile, rather than
    // treating the previous successful compilation as a validation result.
    fs::create_dir_all(root.join("shared/src")).unwrap();
    let dependency_manifest = root.join("shared/Cargo.toml");
    fs::write(
        &dependency_manifest,
        "[package]\nname = \"shared\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        root.join("shared/src/lib.rs"),
        "pub fn value() -> u8 { 1 }\n",
    )
    .unwrap();
    use std::io::Write;
    writeln!(
        fs::OpenOptions::new()
            .append(true)
            .open(root.join("Cargo.toml"))
            .unwrap(),
        "[dependencies]\nshared = {{ path = \"shared\" }}"
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "#[test] fn shared_contract() { assert_eq!(shared::value(), 1); }\n",
    )
    .unwrap();
    assert!(run(root, &["--lib"]).status.success());
    assert!(run(root, &["--worker-budget", "8", "--locked", "--lib"])
        .status
        .success());
    fs::write(
        root.join("shared/src/lib.rs"),
        "pub fn value() -> u8 { 2 }\n",
    )
    .unwrap();
    assert!(!run(root, &["--locked", "--lib"]).status.success());
    fs::write(
        &dependency_manifest,
        "[package]\nname = \"shared\"\nversion = \"0.2.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    let result = run(root, &["--locked", "--lib"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("lock file"));

    let clean = Command::new("cargo")
        .args(["clean", "--locked", "--package", "runner-fixture"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        !clean.status.success(),
        "cache cleanup must not repair a stale lockfile"
    );
    fs::write(
        &dependency_manifest,
        "[package]\nname = \"shared\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();

    // A local dependency must never survive a restored build cache merely because
    // its changed source has an old timestamp. The hosted policy cleans every
    // path package, not only the root validator.
    fs::write(
        root.join("shared/src/lib.rs"),
        "pub fn value() -> u8 { 1 }\n",
    )
    .unwrap();
    assert!(run(root, &["--locked", "--lib"]).status.success());
    fs::write(root.join("shared/src/lib.rs"), "invalid Rust\n").unwrap();
    fs::File::options()
        .write(true)
        .open(root.join("shared/src/lib.rs"))
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH))
        .unwrap();
    let metadata = Command::new("cargo")
        .args(["metadata", "--locked", "--format-version", "1"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(metadata.status.success());
    let mut clean = Command::new("python3")
        .arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(".github/scripts/clean-local-builds.py"),
        )
        .current_dir(root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    clean
        .stdin
        .take()
        .unwrap()
        .write_all(&metadata.stdout)
        .unwrap();
    let clean = clean.wait_with_output().unwrap();
    assert!(
        clean.status.success(),
        "{}",
        String::from_utf8_lossy(&clean.stderr)
    );
    let result = run(root, &["--locked", "--lib"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("cargo test build failed"));
    fs::write(
        root.join("shared/src/lib.rs"),
        "pub fn value() -> u8 { 1 }\n",
    )
    .unwrap();
    assert!(run(root, &["--locked", "--lib"]).status.success());

    // A restored root output must not hide even deliberately backdated bad source.
    // This is the same package-clean boundary used before and after hosted caching.
    fs::write(root.join("src/lib.rs"), "invalid Rust\n").unwrap();
    let file = fs::File::options()
        .write(true)
        .open(root.join("src/lib.rs"))
        .unwrap();
    file.set_times(fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH))
        .unwrap();
    let clean = Command::new("cargo")
        .args(["clean", "--locked", "--package", "runner-fixture"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(clean.status.success());
    let result = run(root, &["--lib"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("cargo test build failed"));
}

#[test]
fn latency_and_recursive_suites_are_isolated_without_skipping_later_failures() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    fs::create_dir(root.join("tests")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"phase-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        root.join("tests/auth_cli.rs"),
        r#"
#[test] fn latency() {
    assert!(!std::path::Path::new("recursive.finished").exists());
    std::fs::write("auth.finished", "yes").unwrap();
    panic!("representative auth regression");
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("tests/pre_commit.rs"),
        r#"
#[test] fn recursive() {
    assert!(std::path::Path::new("auth.finished").exists());
    assert!(!std::path::Path::new("other.finished").exists());
    std::fs::write("recursive.finished", "yes").unwrap();
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("tests/alpha.rs"),
        r#"
#[test] fn other() {
    assert!(std::path::Path::new("recursive.finished").exists());
    std::fs::write("other.finished", "yes").unwrap();
}
"#,
    )
    .unwrap();
    let result = run(
        root,
        &[
            "--test",
            "auth_cli",
            "--test",
            "pre_commit",
            "--test",
            "alpha",
        ],
    );
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("representative auth regression"));
    assert!(
        root.join("other.finished").exists(),
        "all later phases must execute"
    );
}

#[test]
#[cfg(unix)]
fn incomplete_cargo_inventory_cannot_run_a_partial_successful_check() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    let cargo = bin.join("cargo");
    fs::write(&cargo, "#!/bin/sh\nprintf '%s\\n' \"$INVENTORY\"\n").unwrap();
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();
    let executable = bin.join("test-program");
    fs::write(&executable, "#!/bin/sh\ntouch \"$EXECUTED\"\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    let marker = root.join("executed");
    let valid = serde_json::json!({"reason":"compiler-artifact","profile":{"test":true},
        "target":{"name":"fixture","kind":["test"]},"executable":executable});
    let invoke_with_budget = |inventory: String, budget: usize| {
        Command::new(env!("CARGO_BIN_EXE_config-tools"))
            .args([
                "test-parallel",
                "--lib",
                "--worker-budget",
                &budget.to_string(),
            ])
            .current_dir(root)
            .env(
                "PATH",
                format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
            )
            .env("INVENTORY", inventory)
            .env("EXECUTED", &marker)
            .output()
            .unwrap()
    };
    let invoke = |inventory| invoke_with_budget(inventory, 4);
    for invalid in [
        serde_json::json!({"reason":"compiler-artifact","executable":executable}),
        serde_json::json!({"reason":"compiler-artifact","profile":{"test":true},"executable":executable}),
        serde_json::json!({"reason":"compiler-artifact","profile":{"test":true},"target":{"name":"fixture","kind":[]},"executable":executable}),
    ] {
        let result = invoke(format!("{valid}\n{invalid}"));
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("cannot read cargo test inventory")
        );
        assert!(
            !marker.exists(),
            "partial inventories must fail before running tests"
        );
    }
    let result = invoke(valid.to_string());
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        marker.exists(),
        "complete inventory must execute its test program"
    );

    for input in [
        "{}",
        r#"{"packages":[]}"#,
        r#"{"packages":[{"id":"partial"}]}"#,
    ] {
        use std::io::Write;
        let mut clean = Command::new("python3")
            .arg(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(".github/scripts/clean-local-builds.py"),
            )
            .current_dir(root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        clean
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        assert!(
            !clean.wait().unwrap().success(),
            "partial cleanup inventories must fail closed"
        );
    }

    for value in ["0", "invalid"] {
        fs::remove_file(&marker).unwrap_or(());
        let result = Command::new(env!("CARGO_BIN_EXE_config-tools"))
            .args(["test-parallel", "--lib"])
            .current_dir(root)
            .env(
                "PATH",
                format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
            )
            .env("INVENTORY", valid.to_string())
            .env("EXECUTED", &marker)
            .env("RUST_TEST_THREADS", value)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("RUST_TEST_THREADS"));
        assert!(!marker.exists());
    }

    let mut artifacts = Vec::new();
    for name in ["workctl", "reset_cli"] {
        let executable = bin.join(name);
        fs::write(
            &executable,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$EXECUTED-${0##*/}\"\n",
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        artifacts.push(
            serde_json::json!({"reason":"compiler-artifact","profile":{"test":true},
            "target":{"name":name,"kind":["test"]},"executable":executable})
            .to_string(),
        );
    }
    for budget in [2, 3, 4, 5, 8] {
        assert!(invoke_with_budget(artifacts.join("\n"), budget)
            .status
            .success());
        let mut threads = 0;
        for name in ["workctl", "reset_cli"] {
            let args = fs::read_to_string(root.join(format!("executed-{name}"))).unwrap();
            let count: usize = args
                .lines()
                .find_map(|line| line.strip_prefix("--test-threads="))
                .unwrap()
                .parse()
                .unwrap();
            assert!(count > 0);
            threads += count;
        }
        assert!(
            threads <= budget,
            "heavy Git suites exceeded shared worker budget {budget}: {threads}"
        );
    }
}
