//! Controlled local benchmark. All reset operations target disposable clones.
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Instant;

fn configured(cmd: &mut Command) -> &mut Command {
    cmd.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Benchmark")
        .env("GIT_AUTHOR_EMAIL", "benchmark@example.com")
        .env("GIT_COMMITTER_NAME", "Benchmark")
        .env("GIT_COMMITTER_EMAIL", "benchmark@example.com")
        .env("PYTHONDONTWRITEBYTECODE", "1")
}
fn checked(cmd: &mut Command) -> Output {
    let output = configured(cmd).output().expect("launch command");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
fn git(cwd: &Path, args: &[&str]) -> String {
    String::from_utf8(checked(Command::new("git").current_dir(cwd).args(args)).stdout)
        .unwrap()
        .trim()
        .into()
}
fn median(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}
fn main() {
    let mut args = std::env::args().skip(1);
    let mut binary = None;
    let mut baseline = None;
    let mut output = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--binary" => binary = args.next().map(PathBuf::from),
            "--baseline" => baseline = args.next().map(PathBuf::from),
            "--output" => output = args.next().map(PathBuf::from),
            _ => panic!("unknown option {arg}"),
        }
    }
    let binary = binary
        .expect("--binary COMPILED_CONFIG_TOOLS")
        .canonicalize()
        .unwrap();
    let baseline = baseline
        .expect("--baseline OLD_RUNNER_DIRECTORY")
        .canonicalize()
        .unwrap();
    let destination = output.expect("--output RESULT_JSON");
    let root = tempfile::tempdir().unwrap();
    let seed = root.path().join("seed");
    let remote = root.path().join("origin.git");
    let workspace = root.path().join("workspace");
    let shim = root.path().join("bin");
    for dir in [&seed, &workspace, &shim] {
        fs::create_dir(dir).unwrap();
    }
    git(&seed, &["init", "-b", "main"]);
    fs::write(seed.join("file"), "old").unwrap();
    git(&seed, &["add", "."]);
    git(&seed, &["commit", "-m", "old"]);
    git(
        root.path(),
        &[
            "clone",
            "--bare",
            seed.to_str().unwrap(),
            remote.to_str().unwrap(),
        ],
    );
    git(
        &seed,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    let old = git(&seed, &["rev-parse", "HEAD"]);
    let mut repos = vec![];
    for n in 0..12 {
        let repo = workspace.join(format!("repo-{n:02}"));
        git(
            &workspace,
            &["clone", remote.to_str().unwrap(), repo.to_str().unwrap()],
        );
        repos.push(repo);
    }
    fs::write(seed.join("file"), "new").unwrap();
    git(&seed, &["commit", "-am", "new"]);
    git(&seed, &["push", "origin", "main"]);
    let new = git(&seed, &["rev-parse", "HEAD"]);
    let real_git = String::from_utf8(
        Command::new("sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    fs::write(shim.join("git"),format!("#!/bin/sh\nfor arg in \"$@\"; do if [ \"$arg\" = fetch ]; then sleep 0.25; fi; done\nexec '{}' \"$@\"\n",real_git.trim())).unwrap();
    fs::set_permissions(shim.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:{}", shim.display(), std::env::var("PATH").unwrap());
    let mut samples = std::collections::BTreeMap::<String, Vec<f64>>::new();
    for order in [
        ["home-4", "rust-1", "rust-4"],
        ["rust-4", "home-4", "rust-1"],
        ["rust-1", "rust-4", "home-4"],
    ] {
        for mode in order {
            for repo in &repos {
                git(repo, &["reset", "--hard", &old]);
            }
            let mut command = if mode == "home-4" {
                let mut c = Command::new("python3");
                c.arg(baseline.join("home_reset.py")).args([
                    "--root",
                    workspace.to_str().unwrap(),
                    "--jobs",
                    "4",
                    "--retries",
                    "1",
                    "--no-resolve-case-conflicts",
                ]);
                c
            } else {
                let mut c = Command::new(&binary);
                c.args([
                    "reset-to-origin",
                    "--jobs",
                    if mode == "rust-1" { "1" } else { "4" },
                    "--attempts",
                    "1",
                    workspace.to_str().unwrap(),
                ]);
                c
            };
            command.env("PATH", &path);
            let started = Instant::now();
            checked(&mut command);
            let seconds = started.elapsed().as_secs_f64();
            for repo in &repos {
                assert_eq!(git(repo, &["rev-parse", "HEAD"]), new);
            }
            println!("{mode}: {seconds:.3}s; all 12 HEADs verified");
            samples.entry(mode.into()).or_default().push(seconds);
        }
    }
    let mut startup = vec![];
    for _ in 0..10 {
        let start = Instant::now();
        checked(Command::new(&binary).args(["reset-to-origin", "--help"]));
        startup.push(start.elapsed().as_secs_f64());
    }
    let start = Instant::now();
    checked(Command::new(&binary).args(["reset-to-origin", "--list", workspace.to_str().unwrap()]));
    let discovery = start.elapsed().as_secs_f64();
    let medians: std::collections::BTreeMap<_, _> = samples
        .iter()
        .map(|(name, samples)| (name.clone(), median(samples.clone())))
        .collect();
    let result = serde_json::json!({"repositories":12,"fetch_delay_seconds":0.25,"baseline":"supervised home_reset_to_origin, case recovery disabled","samples":samples,"median_seconds":medians,"speedup_rust4_vs_home4":medians["home-4"]/medians["rust-4"],"speedup_rust4_vs_rust1":medians["rust-1"]/medians["rust-4"],"rust_help_median_seconds":median(startup),"rust_discovery_seconds":discovery});
    fs::write(destination, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
