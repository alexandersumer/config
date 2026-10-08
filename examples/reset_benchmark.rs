//! Reproducible worker-count comparison using only disposable Git checkouts.
use clap::Parser;
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Instant;

#[derive(Parser)]
#[command(about = "Compare reset worker counts on disposable local repositories")]
struct Args {
    /// Compiled workctl executable
    #[arg(long)]
    binary: PathBuf,
    /// JSON result destination
    #[arg(long)]
    output: PathBuf,
    /// Worker counts to compare
    #[arg(long, value_delimiter = ',', default_value = "1,4,8,16,24,32", value_parser = clap::value_parser!(u32).range(1..=32))]
    jobs: Vec<u32>,
    /// Number of disposable repositories
    #[arg(long, default_value_t = 64, value_parser = clap::value_parser!(u32).range(1..=512))]
    repositories: u32,
    /// Trials per worker count, with rotated execution order
    #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u32).range(1..=10))]
    runs: u32,
    /// Controlled latency added to each local fetch, in seconds
    #[arg(long, default_value = "0.25", value_parser = delay)]
    fetch_delay: f64,
}
fn delay(value: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && (0.0..=60.0).contains(n))
        .ok_or_else(|| "expected a finite delay from 0 to 60 seconds".into())
}
fn configured(cmd: &mut Command) -> &mut Command {
    cmd.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Benchmark")
        .env("GIT_AUTHOR_EMAIL", "benchmark@example.com")
        .env("GIT_COMMITTER_NAME", "Benchmark")
        .env("GIT_COMMITTER_EMAIL", "benchmark@example.com")
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
fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}
fn main() {
    let args = Args::parse();
    let root = tempfile::tempdir().unwrap();
    let binary = root.path().join("workctl");
    fs::copy(args.binary, &binary).expect("copy standalone executable");
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
    for n in 0..args.repositories {
        let repo = workspace.join(format!("repo-{n:03}"));
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
    let escaped = real_git.trim().replace('\'', "'\\''");
    fs::write(shim.join("git"), format!("#!/bin/sh\nfor arg in \"$@\"; do if [ \"$arg\" = fetch ]; then sleep {}; fi; done\nexec '{}' \"$@\"\n", args.fetch_delay, escaped)).unwrap();
    fs::set_permissions(shim.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:{}", shim.display(), std::env::var("PATH").unwrap());
    let mut samples = BTreeMap::<u32, Vec<f64>>::new();
    for trial in 0..args.runs as usize {
        for offset in 0..args.jobs.len() {
            let jobs = args.jobs[(trial + offset) % args.jobs.len()];
            for repo in &repos {
                git(repo, &["reset", "--hard", &old]);
            }
            let mut command = Command::new(&binary);
            command
                .args([
                    "git",
                    "reset",
                    "--jobs",
                    &jobs.to_string(),
                    "--attempts",
                    "1",
                ])
                .arg(&workspace)
                .env("PATH", &path)
                .env("TMPDIR", root.path());
            let started = Instant::now();
            checked(&mut command);
            let seconds = started.elapsed().as_secs_f64();
            for repo in &repos {
                assert_eq!(git(repo, &["rev-parse", "HEAD"]), new);
                assert_eq!(git(repo, &["rev-parse", "@{upstream}"]), new);
                assert!(git(
                    repo,
                    &[
                        "--no-optional-locks",
                        "status",
                        "--porcelain",
                        "--untracked-files=no"
                    ]
                )
                .is_empty());
                let backups = git(
                    repo,
                    &[
                        "for-each-ref",
                        "--format=%(objectname)",
                        "refs/home-reset-backups/",
                    ],
                );
                assert!(!backups.is_empty());
                assert!(backups.lines().all(|oid| oid == old));
            }
            println!(
                "{jobs} workers: {seconds:.3}s; all {} checkouts verified",
                repos.len()
            );
            samples.entry(jobs).or_default().push(seconds);
        }
    }
    let medians: BTreeMap<_, _> = samples
        .iter()
        .map(|(jobs, samples)| (*jobs, median(samples.clone())))
        .collect();
    let result = serde_json::json!({"repositories":args.repositories,"fetch_delay_seconds":args.fetch_delay,"samples":samples,"median_seconds":medians,"scope":"controlled local latency; not live remote performance","verified":"HEAD, upstream, tracked state and recovery tips"});
    fs::write(args.output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
