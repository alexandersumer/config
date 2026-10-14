//! Explicit real-provider E2E. Never runs in credential-free CI by accident.
//! Run: cargo test --locked --test auth_live -- --ignored --nocapture
use serde_json::Value;
use std::process::{Command, Stdio};
const BIN: &str = env!("CARGO_BIN_EXE_workctl");
fn result(args: &[&str], expected: i32) -> Value {
    assert!(
        std::env::var_os("AUTH_FIXTURE").is_none(),
        "Live proof cannot use provider simulators"
    );
    let out = Command::new(BIN)
        .args(args)
        .stderr(Stdio::inherit())
        .output()
        .expect("launch real workctl");
    // Never dump provider or child output on assertion failure. Allowed login
    // prompts remain live on stderr while stdout stays one JSON document.
    assert_eq!(
        out.status.code(),
        Some(expected),
        "Unexpected live workctl exit status"
    );
    serde_json::from_slice(&out.stdout).expect("schema-versioned auth result")
}
fn reuse_and_exec(options: &[&str], targets: usize) {
    let mut args = vec!["auth", "preflight"];
    args.extend_from_slice(options);
    args.push("--json");
    let refresh = result(&args, 0);
    assert_eq!(refresh["status"], "completed");
    assert_eq!(refresh["targets"].as_array().unwrap().len(), targets);
    args.push("--check");
    let cached = result(&args, 0);
    assert!(cached["targets"]
        .as_array()
        .unwrap()
        .iter()
        .all(|t| t["status"] == "valid"));
    let mut args = vec!["auth", "exec"];
    args.extend_from_slice(options);
    args.extend_from_slice(&[
        "--check",
        "--",
        "/bin/sh",
        "-c",
        "printf live-auth-exec; exit 37",
    ]);
    let command = Command::new(BIN).args(args).output().unwrap();
    assert_eq!(command.status.code(), Some(37));
    assert_eq!(command.stdout, b"live-auth-exec");
}
#[test]
#[ignore = "requires real bb/TWG installations, shared credentials and network"]
fn real_code_provider_preflight_cache_reuse_and_wrapped_command() {
    reuse_and_exec(&["--profile", "code"], 2);
    println!("Real Bitbucket/TWG: preflight, cache reuse, exact child stdout/exit verified");
}
#[test]
#[ignore = "requires real Atlas, shared credentials, network, and browser/MFA"]
fn real_slauth_preflight_cache_reuse_and_ttl_policy() {
    // Explicit config permits a narrow non-MFA service proof without claiming
    // that a different service's MFA enrollment was verified.
    let config = std::env::var("WORKCTL_AUTH_LIVE_CONFIG").ok();
    let profile = std::env::var("WORKCTL_AUTH_LIVE_PROFILE").unwrap_or_else(|_| "logs".into());
    let mut options = vec!["--profile", profile.as_str()];
    if let Some(config) = &config {
        options.extend_from_slice(&["--config", config.as_str()]);
    }
    reuse_and_exec(&options, 1);
    let mut args = vec!["auth", "preflight"];
    args.extend_from_slice(&options);
    args.extend_from_slice(&["--check", "--min-valid-for", "7200", "--json"]);
    let impossible = result(&args, 1);
    assert_eq!(impossible["targets"][0]["status"], "blocked");
    println!("Real SLAuth ({profile}): preflight, cache reuse, wrapped command, impossible lifetime verified");
}
