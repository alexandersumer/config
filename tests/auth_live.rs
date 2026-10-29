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
    assert!(refresh["targets"]
        .as_array()
        .unwrap()
        .iter()
        .all(|t| t["status"] == "refreshed"
            || (t["target"] == "twg-bitbucket" && t["status"] == "valid")));
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
    reuse_and_exec(&["--profile", "code"], 3);
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

#[test]
#[ignore = "requires real bb/TWG/Atlas installations, shared caches and network; never renews"]
fn real_work_cache_checks_all_eight_targets_without_renewal() {
    let checked = result(&["auth", "preflight", "--check", "--json"], 0);
    let targets = checked["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 8);
    assert!(targets.iter().all(|t| t["status"] == "valid"));
    for audience in [
        "splunk.paas-inf.net",
        "risk-mitigation-service",
        "pollinator",
        "mlp-config",
        "phobos",
    ] {
        assert!(targets.iter().any(|t| t["target"]
            .as_str()
            .unwrap()
            .contains(&format!(":{audience}:"))));
    }
    println!("Real work profile: all eight current provider credentials verified without explicit renewal");
}

#[test]
#[ignore = "requires real TWG connectivity; accepts an explicitly blocked missing or invalid token"]
fn real_twg_bitbucket_reports_independent_readiness_without_renewal() {
    assert!(
        std::env::var_os("AUTH_FIXTURE").is_none(),
        "Live proof cannot use provider simulators"
    );
    let out = Command::new(BIN)
        .args([
            "auth",
            "preflight",
            "--profile",
            "twg-bitbucket",
            "--check",
            "--json",
        ])
        .output()
        .unwrap();
    let r: Value = serde_json::from_slice(&out.stdout).expect("auth report");
    assert_eq!(r["targets"][0]["target"], "twg-bitbucket");
    let status = r["targets"][0]["status"].as_str().unwrap();
    assert!(
        matches!(status, "valid" | "blocked"),
        "unexpected TWG provider failure"
    );
    assert_eq!(out.status.success(), status == "valid");
    if status == "blocked" {
        assert!(r["targets"][0]["action"]
            .as_str()
            .unwrap()
            .contains("twg setup bitbucket --force"));
    }
    println!("Real TWG Bitbucket status: {status}; no explicit renewal");
}

#[test]
#[ignore = "requires a real valid default-cache Kerberos TGT; never renews"]
fn real_kerberos_cache_and_exact_principal_are_checked_without_renewal() {
    let checked = result(
        &[
            "auth",
            "preflight",
            "--profile",
            "runtime",
            "--check",
            "--json",
        ],
        0,
    );
    let principal = checked["targets"][0]["account"]
        .as_str()
        .expect("verified principal");
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("auth.json");
    for (selected, expected, status) in [
        (principal, 0, "valid"),
        ("workctl-proof-wrong-principal@EXAMPLE.INVALID", 1, "failed"),
    ] {
        std::fs::write(
            &config,
            serde_json::json!({"schema_version":1,
            "profiles":{"exact":[{"provider":"kerberos","principal":selected}]}})
            .to_string(),
        )
        .unwrap();
        let checked = result(
            &[
                "auth",
                "preflight",
                "--config",
                config.to_str().unwrap(),
                "--profile",
                "exact",
                "--check",
                "--json",
            ],
            expected,
        );
        assert_eq!(checked["targets"][0]["status"], status);
    }
    println!("Real Kerberos: default TGT, exact identity, and wrong-identity refusal verified without renewal");
}
