//! Real workctl CLI, OS process groups, pipes, locks, argv and files. The external
//! provider CLIs are simulated; real account/cache proof lives in auth_live.rs.
use serde_json::{json, Value};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};
const BIN: &str = env!("CARGO_BIN_EXE_workctl");
const SECRET: &str = "AUTH_SECRET_MUST_NEVER_APPEAR";
struct Fixture {
    root: tempfile::TempDir,
}
impl Fixture {
    fn new(state: Value) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("bin")).unwrap();
        let script = root.path().join("provider.py");
        fs::write(&script, include_str!("fixtures/auth_provider.py")).unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        for name in ["bb", "twg", "atlas"] {
            symlink(&script, root.path().join("bin").join(name)).unwrap();
        }
        let f = Self { root };
        f.state(state);
        f
    }
    fn state(&self, state: Value) {
        fs::write(self.root.path().join("state.json"), state.to_string()).unwrap();
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(BIN);
        c.args(args)
            .env("AUTH_FIXTURE", self.root.path())
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.root.path().join("bin").display()),
            )
            .env("TMPDIR", self.root.path())
            .env("TERM", "dumb")
            .env("NO_COLOR", "1")
            .env("COLUMNS", "80")
            .stdin(Stdio::null());
        c
    }
    fn calls(&self) -> Vec<Value> {
        fs::read_to_string(self.root.path().join("calls.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect()
    }
    fn run(&self, extra: &[&str], expected: i32) -> Value {
        let mut c = self.command(&["auth", "preflight", "--reuse-valid", "--json"]);
        let out = c.args(extra).output().unwrap();
        assert_eq!(out.status.code(), Some(expected), "{}", text(&out));
        assert!(!text(&out).contains(SECRET), "{}", text(&out));
        assert!(!out.stdout.contains(&0x1b));
        serde_json::from_slice(&out.stdout).expect("one JSON result")
    }
    fn profile(&self, targets: Value) -> String {
        let path = self.root.path().join("auth.json");
        fs::write(
            &path,
            json!({"schema_version":1,"profiles":{"work":targets}}).to_string(),
        )
        .unwrap();
        path.to_str().unwrap().to_owned()
    }
}
fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}
fn wait_file(path: &std::path::Path) {
    let end = Instant::now() + Duration::from_secs(5);
    while !path.exists() {
        assert!(Instant::now() < end, "fixture did not start");
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn alive(pid: i32) -> bool {
    // Zombies are inert; ps is a portable observation across Linux/macOS.
    let out = Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .unwrap();
    out.status.success()
        && !String::from_utf8_lossy(&out.stdout).trim().is_empty()
        && !String::from_utf8_lossy(&out.stdout).trim().starts_with('Z')
}
#[test]
fn valid_cache_reuse_is_read_only_and_json_never_contains_credentials() {
    let f = Fixture::new(json!({}));
    for _ in 0..2 {
        let r = f.run(&[], 0);
        assert_eq!(r["status"], "completed");
        assert_eq!(r["targets"][0]["status"], "valid");
        assert_eq!(r["targets"][1]["remaining_seconds"], Value::Null);
        assert!(r["targets"][1]["note"]
            .as_str()
            .unwrap()
            .contains("No unattended"));
        assert_eq!(r["targets"][2]["status"], "valid");
        assert_eq!(r["duration_guaranteed"], false);
    }
    assert_eq!(f.calls().len(), 16);
    assert!(f.calls().iter().all(|c| c[1][1] == "status"
        || c[1][0] == "workspace"
        || c[1][0] == "doctor"
        || c[1].as_array().unwrap().contains(&json!("--cached"))));
}
#[test]
fn all_missing_credentials_are_refreshed_once_and_normal_cache_is_rechecked() {
    let f = Fixture::new(json!({"bb_valid":false,"twg_valid":false,"atlas_valid":false}));
    let r = f.run(&[], 0);
    assert!(r["targets"]
        .as_array()
        .unwrap()
        .iter()
        .all(|v| v["status"] == "refreshed"));
    let calls = f.calls();
    assert_eq!(
        calls
            .iter()
            .filter(|c| c[0] == "bb" && c[1][1] == "login")
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|c| c[0] == "twg" && c[1][1] == "refresh")
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|c| c[0] == "atlas" && c[1].as_array().unwrap().contains(&json!("--force")))
            .count(),
        5
    );
    assert_eq!(
        calls
            .iter()
            .filter(|c| c[0] == "atlas" && c[1].as_array().unwrap().contains(&json!("--cached")))
            .count(),
        10
    );
    let before = calls.len();
    f.run(&[], 0);
    assert_eq!(f.calls().len() - before, 8);
}
#[test]
fn silent_oauth_refresh_succeeds_without_browser_in_noninteractive_mode() {
    let f = Fixture::new(json!({"bb_minutes":1,"bb_silent":true,"twg_valid":false}));
    f.run(&["--profile", "code", "--non-interactive"], 0);
    assert!(!f.calls().iter().any(|c| c[1][1] == "login"));
}
#[test]
fn short_lived_active_account_is_preserved_during_login() {
    let f = Fixture::new(json!({"bb_minutes":1}));
    f.run(&["--profile", "code"], 0);
    let calls = f.calls();
    let login = calls
        .iter()
        .find(|c| c[0] == "bb" && c[1][1] == "login")
        .unwrap();
    assert_eq!(login[1], json!(["auth", "login", "--account", "office"]));
}
#[test]
fn check_only_never_makes_any_refresh_or_login_call() {
    let f = Fixture::new(json!({"bb_valid":false,"twg_valid":false,"atlas_valid":false}));
    let r = f.run(&["--check"], 1);
    assert_eq!(r["status"], "blocked");
    assert_eq!(f.calls().len(), 7);
}
#[test]
fn noninteractive_blocks_mfa_instead_of_launching_a_browser() {
    let f = Fixture::new(json!({"atlas_valid":false}));
    f.run(&["--profile", "logs", "--non-interactive"], 1);
    assert_eq!(f.calls().len(), 1);
}
#[test]
fn refresh_success_without_cached_auth_is_not_readiness_and_does_not_loop() {
    let f = Fixture::new(json!({"atlas_valid":false,"refresh_stale":true}));
    let r = f.run(&["--profile", "logs"], 1);
    assert!(r["targets"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("after one refresh"));
    assert_eq!(f.calls().len(), 3);
}
#[test]
fn provider_ttl_cannot_silently_claim_several_hours() {
    let f = Fixture::new(json!({"ttl":3600}));
    let r = f.run(&["--profile", "logs", "--min-valid-for", "7200"], 1);
    assert_eq!(r["targets"][0]["status"], "blocked");
    assert!(r["targets"][0]["remaining_seconds"].as_u64().unwrap() <= 3600);
    assert_eq!(f.calls().len(), 3);
}
#[test]
fn malformed_permission_and_outage_errors_do_not_trigger_auth_refresh() {
    for mode in ["malformed", "permission", "outage"] {
        let f = Fixture::new(json!({"mode":mode}));
        let r = f.run(&[], 1);
        assert_eq!(f.calls().len(), 7);
        assert!(r["targets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["status"] == "failed"));
    }
}
#[test]
fn jwt_audience_expiry_and_not_before_are_verified() {
    for state in [json!({"aud":"another-service"}), json!({"nbf":u64::MAX})] {
        let f = Fixture::new(state);
        f.run(&["--profile", "logs"], 1);
        assert_eq!(f.calls().len(), 1);
    }
    let f = Fixture::new(json!({"ttl":-60}));
    f.run(&["--profile", "logs"], 0);
    assert_eq!(f.calls().len(), 3);
}
#[test]
fn configuration_is_validated_before_any_provider_calls() {
    for targets in [
        json!([]),
        json!([{"provider":"arbitrary","command":"echo secret"}]),
        json!([{"provider":"twg","secret":SECRET}]),
        json!([{"provider":"twg"},{"provider":"twg"}]),
        json!([{"provider":"slauth","audience":"--evil"}]),
        json!([{"provider":"slauth","audience":"valid","environment":"production"}]),
    ] {
        let f = Fixture::new(json!({}));
        let path = f.profile(targets);
        f.run(&["--config", &path], 1);
        assert!(f.calls().is_empty());
    }
}
#[test]
fn custom_descriptor_preserves_exact_environment_groups_and_mfa() {
    let f = Fixture::new(json!({"aud":"test-service"}));
    let path=f.profile(json!([{"provider":"slauth","audience":"test-service","environment":"staging","groups":["team-z","team-a"],"mfa":false}]));
    f.run(&["--config", &path], 0);
    assert_eq!(
        f.calls()[0][1],
        json!([
            "slauth",
            "token",
            "--aud",
            "test-service",
            "--env",
            "staging",
            "--output",
            "jwt",
            "--groups",
            "team-a,team-z",
            "--cached"
        ])
    );
}
#[test]
fn byte_safe_exec_preserves_argv_environment_stdin_and_exit_status() {
    let f = Fixture::new(json!({}));
    let out = f
        .command(&[
            "auth",
            "exec",
            "--profile",
            "code",
            "--",
            "/bin/sh",
            "-c",
            "printf '%s' \"$1\"; test \"$AUTH_FIXTURE\"; exit 37",
            "runner",
            "spaces; $(do-not-run) --json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(37));
    assert_eq!(out.stdout, b"spaces; $(do-not-run) --json");
    assert!(String::from_utf8_lossy(&out.stderr).contains("Authentication preflight"));
    assert!(!text(&out).contains(SECRET));
    use std::io::Write;
    let mut c = f.command(&["auth", "exec", "--profile", "code", "--", "/bin/cat"]);
    c.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"input preserved\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.stdout, b"input preserved\n");
}
#[test]
fn blocked_auth_and_invalid_usage_never_execute_the_child() {
    let f = Fixture::new(json!({"bb_valid":false}));
    let marker = f.root.path().join("child");
    let out = f
        .command(&[
            "auth",
            "exec",
            "--profile",
            "code",
            "--check",
            "--",
            "/usr/bin/touch",
            marker.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(!marker.exists());
    for args in [
        vec!["auth", "exec"],
        vec!["auth", "preflight", "--timeout", "0"],
        vec![
            "auth",
            "exec",
            "--json",
            "--",
            "/usr/bin/touch",
            marker.to_str().unwrap(),
        ],
    ] {
        let out = f.command(&args).output().unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(!marker.exists());
    }
}
#[test]
fn oversized_provider_output_is_bounded_and_secret_free() {
    let f = Fixture::new(json!({"mode":"flood"}));
    let r = f.run(&["--profile", "logs"], 1);
    assert!(r["targets"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("safety limit"));
    assert_eq!(f.calls().len(), 1);
}
#[test]
fn deadline_and_cancellation_clean_provider_descendants_and_release_lock() {
    for interrupt in [false, true] {
        let f = Fixture::new(json!({"mode":"hang"}));
        let mut child = f
            .command(&[
                "auth",
                "preflight",
                "--profile",
                "logs",
                "--json",
                "--timeout",
                "5",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        wait_file(&f.root.path().join("started"));
        if interrupt {
            unsafe {
                libc::kill(child.id() as i32, libc::SIGINT);
            }
        }
        let began = Instant::now();
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            assert!(began.elapsed() < Duration::from_secs(7));
            std::thread::sleep(Duration::from_millis(10));
        }
        let out = child.wait_with_output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(if interrupt { 130 } else { 1 }),
            "{}",
            text(&out)
        );
        let pid = fs::read_to_string(f.root.path().join("descendant"))
            .unwrap()
            .parse()
            .unwrap();
        assert!(!alive(pid));
        f.state(json!({}));
        f.run(&["--profile", "logs"], 0);
    }
}
#[test]
fn successful_leader_exit_still_cleans_descendants() {
    let f = Fixture::new(json!({"mode":"leader_exit"}));
    f.run(&["--profile", "logs"], 0);
    let pid = fs::read_to_string(f.root.path().join("descendant"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(!alive(pid));
}
#[test]
fn coordination_serializes_refreshes_and_rechecks_after_waiting() {
    let f = Fixture::new(json!({"atlas_valid":false,"prompt":true}));
    let first = f
        .command(&[
            "auth",
            "preflight",
            "--reuse-valid",
            "--profile",
            "logs",
            "--json",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_file(&f.root.path().join("started"));
    let mut second = f
        .command(&[
            "auth",
            "preflight",
            "--reuse-valid",
            "--profile",
            "logs",
            "--json",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert!(second.try_wait().unwrap().is_none());
    assert_eq!(f.calls().len(), 2);
    fs::write(f.root.path().join("release"), "").unwrap();
    assert!(first.wait_with_output().unwrap().status.success());
    assert!(second.wait_with_output().unwrap().status.success());
    assert_eq!(
        f.calls()
            .iter()
            .filter(|c| c[1].as_array().unwrap().contains(&json!("--force")))
            .count(),
        1
    );
}
#[test]
fn activation_prompt_is_visible_before_provider_completes_and_no_secret_leaks() {
    use std::io::{BufRead, BufReader};
    let f = Fixture::new(json!({"atlas_valid":false,"prompt":true}));
    let mut child = f
        .command(&["auth", "preflight", "--profile", "logs", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stderr = child.stderr.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut all = String::new();
        for line in BufReader::new(stderr).lines() {
            let line = line.unwrap();
            if line.contains("ABCD1234") {
                tx.send(()).unwrap();
            }
            all.push_str(&line);
        }
        all
    });
    // Measure forwarding after the provider emits its prompt, not cold CLI startup.
    wait_file(&f.root.path().join("started"));
    let visible = rx.recv_timeout(Duration::from_secs(3));
    fs::write(f.root.path().join("release"), "").unwrap();
    let out = child.wait_with_output().unwrap();
    let stderr = reader.join().unwrap();
    assert!(visible.is_ok(), "prompt was not live");
    assert!(out.status.success());
    assert!(!stderr.contains(SECRET));
    assert!(!String::from_utf8_lossy(&out.stdout).contains(SECRET));
}
#[test]
fn unsafe_symlink_fifo_or_public_coordination_file_is_refused_without_mutation() {
    let f = Fixture::new(json!({}));
    let lock = f
        .root
        .path()
        .join(format!("workctl-auth-{}.lock", unsafe { libc::geteuid() }));
    let victim = f.root.path().join("victim");
    fs::write(&victim, "preserve").unwrap();
    symlink(&victim, &lock).unwrap();
    f.run(&[], 1);
    assert_eq!(fs::read_to_string(&victim).unwrap(), "preserve");
    assert!(f.calls().is_empty());
    fs::remove_file(&lock).unwrap();
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let path = CString::new(lock.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    f.run(&[], 1);
    assert!(f.calls().is_empty());
    fs::remove_file(&lock).unwrap();
    fs::write(&lock, "").unwrap();
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o644)).unwrap();
    f.run(&[], 1);
    assert!(f.calls().is_empty());
}
#[test]
fn missing_provider_is_a_structured_failure_and_other_targets_continue() {
    let f = Fixture::new(json!({}));
    fs::remove_file(f.root.path().join("bin/bb")).unwrap();
    let r = f.run(&[], 1);
    assert_eq!(r["targets"][0]["status"], "failed");
    assert_eq!(r["targets"][1]["status"], "valid");
    assert_eq!(r["targets"][2]["status"], "valid");
}

#[test]
fn credentials_are_rechecked_for_age_at_command_handoff() {
    let f = Fixture::new(json!({"bb_minutes":1,"twg_delay":2}));
    let r = f.run(&["--profile", "code", "--min-valid-for", "60"], 1);
    assert_eq!(r["targets"][0]["status"], "blocked");
    assert!(r["targets"][0]["remaining_seconds"].as_u64().unwrap() < 60);
    assert!(r["targets"][0]["reason"].as_str().unwrap().contains("aged"));
}
#[test]
fn special_or_oversized_profile_files_fail_promptly_without_provider_calls() {
    let f = Fixture::new(json!({}));
    let path = f.root.path().join("profile");
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let start = Instant::now();
    f.run(&["--config", path.to_str().unwrap()], 1);
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(f.calls().is_empty());
    fs::remove_file(&path).unwrap();
    fs::write(&path, vec![b'x'; 65537]).unwrap();
    f.run(&["--config", path.to_str().unwrap()], 1);
    assert!(f.calls().is_empty());
}

#[test]
fn exec_preserves_non_utf8_arguments_and_signal_status() {
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::process::ExitStatusExt;
    let f = Fixture::new(json!({}));
    let out = f
        .command(&[
            "auth",
            "exec",
            "--profile",
            "code",
            "--",
            "/usr/bin/printf",
            "%s",
        ])
        .arg(std::ffi::OsString::from_vec(b"arg-\xff".to_vec()))
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(out.stdout, b"arg-\xff");
    let out = f
        .command(&[
            "auth",
            "exec",
            "--profile",
            "code",
            "--",
            "/bin/sh",
            "-c",
            "kill -TERM $$",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.signal(), Some(libc::SIGTERM));
}
#[test]
fn coordination_wait_has_a_deadline_and_does_not_invoke_providers() {
    let f = Fixture::new(json!({"mode":"hang"}));
    let mut first = f
        .command(&["auth", "preflight", "--profile", "logs", "--timeout", "10"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_file(&f.root.path().join("started"));
    let r = f.run(&["--profile", "logs", "--timeout", "1"], 1);
    assert!(r["error"]
        .as_str()
        .unwrap()
        .contains("coordination deadline"));
    assert_eq!(f.calls().len(), 1);
    unsafe {
        libc::kill(first.id() as i32, libc::SIGTERM);
    }
    let status = first.wait().unwrap();
    assert_eq!(status.code(), Some(130));
    let pid = fs::read_to_string(f.root.path().join("descendant"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(!alive(pid));
}

#[test]
fn inconclusive_refresh_checks_the_cache_once_without_reissuing() {
    let f = Fixture::new(json!({"atlas_valid":false,"refresh_hang":true}));
    let r = f.run(&["--profile", "logs", "--login-timeout", "3"], 0);
    assert_eq!(r["targets"][0]["status"], "refreshed");
    assert_eq!(f.calls().len(), 3);
    assert_eq!(
        f.calls()
            .iter()
            .filter(|c| c[1].as_array().unwrap().contains(&json!("--force")))
            .count(),
        1
    );
}

#[test]
fn closed_exec_report_prevents_launch_and_closed_auth_prompt_does_not_panic() {
    let f = Fixture::new(json!({}));
    let marker = f.root.path().join("child");
    let mut child = f
        .command(&[
            "auth",
            "exec",
            "--profile",
            "code",
            "--",
            "/usr/bin/touch",
            marker.to_str().unwrap(),
        ])
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    drop(child.stderr.take());
    assert_eq!(child.wait().unwrap().code(), Some(1));
    assert!(!marker.exists());
    f.state(json!({"atlas_valid":false}));
    let mut child = f
        .command(&["auth", "preflight", "--profile", "logs", "--json"])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stderr.take());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["targets"][0]["status"], "failed");
    assert!(!text(&out).contains(SECRET));
}

#[test]
fn revoked_bitbucket_credentials_require_refresh_despite_unexpired_cache() {
    let f = Fixture::new(json!({"bb_revoked":true}));
    let r = f.run(&["--profile", "code"], 0);
    assert_eq!(r["targets"][0]["status"], "refreshed");
    assert_eq!(
        f.calls()
            .iter()
            .filter(|c| c[0] == "bb" && c[1][1] == "login")
            .count(),
        1
    );
    assert!(f
        .calls()
        .iter()
        .any(|c| c[0] == "bb" && c[1] == json!(["auth", "login", "--account", "office"])));
    f.state(json!({"bb_revoked":true}));
    f.run(&["--profile", "code", "--check"], 1);
}
#[test]
fn bitbucket_api_outage_and_inactive_named_account_do_not_launch_login() {
    let f = Fixture::new(json!({"bb_api_outage":true}));
    f.run(&["--profile", "code"], 1);
    assert!(!f
        .calls()
        .iter()
        .any(|c| c[1][1] == "login" || c[1][1] == "token"));
    let f = Fixture::new(json!({"bb_inactive":true,"bb_other_active":true}));
    let path = f.profile(json!([{"provider":"bitbucket","account":"office"}]));
    f.run(&["--config", &path], 1);
    assert_eq!(f.calls().len(), 1);
}

#[test]
fn valid_login_cannot_repair_a_group_policy_denial() {
    let f = Fixture::new(json!({"atlas_valid":false,"refresh_denied":true}));
    let r = f.run(&["--profile", "logs"], 1);
    assert_eq!(r["targets"][0]["status"], "failed");
    assert!(r["targets"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("service or group policy"));
    assert_eq!(
        f.calls()
            .iter()
            .filter(|c| c[1].as_array().unwrap().contains(&json!("--force")))
            .count(),
        1
    );
    assert_eq!(f.calls().len(), 3);
}

#[test]
fn failed_issuance_exit_still_checks_the_independent_cache_once() {
    let f = Fixture::new(json!({"atlas_valid":false,"refresh_cache_then_fail":true}));
    let r = f.run(&["--profile", "logs"], 0);
    assert_eq!(r["targets"][0]["status"], "refreshed");
    assert_eq!(f.calls().len(), 3);
}
#[test]
fn issuance_outage_is_not_mislabeled_as_missing_human_mfa() {
    let f = Fixture::new(json!({"atlas_valid":false,"refresh_outage":true}));
    let r = f.run(&["--profile", "logs"], 1);
    assert_eq!(r["targets"][0]["status"], "failed");
    assert!(r["targets"][0]["reason"]
        .as_str()
        .unwrap()
        .contains("issuance failed"));
    assert_eq!(f.calls().len(), 3);
}

#[test]
fn default_preflight_forces_every_valid_provider_once_on_every_invocation() {
    let f = Fixture::new(json!({}));
    for _ in 0..2 {
        let out = f
            .command(&["auth", "preflight", "--json"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "{}", text(&out));
        assert!(!text(&out).contains(SECRET));
        let result: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert!(result["targets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["status"] == "refreshed"));
    }
    for provider in ["bb", "twg", "atlas"] {
        let calls = f.calls();
        let renewals: Vec<_> = calls
            .iter()
            .filter(|c| {
                c[0] == provider
                    && (c[1][1] == "login"
                        || c[1][1] == "refresh"
                        || c[1].as_array().unwrap().contains(&json!("--force")))
            })
            .collect();
        assert_eq!(
            renewals.len(),
            if provider == "atlas" { 10 } else { 2 },
            "{provider} must renew exactly once per invocation"
        );
    }
    assert!(f
        .calls()
        .iter()
        .filter(|c| c[0] == "bb" && c[1][1] == "login")
        .all(|c| c[1] == json!(["auth", "login", "--account", "office"])));
}
#[test]
fn forced_preflight_does_not_hide_failed_renewal_behind_valid_cache() {
    let f = Fixture::new(json!({"refresh_fails":true}));
    let out = f
        .command(&["auth", "preflight", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(!text(&out).contains(SECRET));
    let result: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(result["targets"]
        .as_array()
        .unwrap()
        .iter()
        .all(|v| v["status"] != "refreshed" && v["status"] != "valid"));
    assert_eq!(
        f.calls()
            .iter()
            .filter(|c| c[0] == "atlas" && c[1].as_array().unwrap().contains(&json!("--force")))
            .count(),
        5
    );
}
#[test]
fn forced_noninteractive_does_not_claim_existing_credentials_were_renewed() {
    let f = Fixture::new(json!({}));
    let out = f
        .command(&["auth", "preflight", "--non-interactive", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let result: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["targets"][0]["status"], "failed");
    assert_eq!(result["targets"][1]["status"], "refreshed");
    assert_eq!(result["targets"][2]["status"], "blocked");
    assert!(!f.calls().iter().any(|c| c[1][1] == "login"
        || c[1].as_array().unwrap().contains(&json!("--force")) && c[0] == "atlas"));
}

#[test]
fn forced_twg_requires_confirmed_renewal_not_a_skipped_refresh() {
    for status in ["not_due", "skipped", "fresh", "unknown"] {
        let f = Fixture::new(json!({"twg_refresh_status":status}));
        let out = f
            .command(&["auth", "preflight", "--profile", "code", "--json"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1), "{status}");
        let result: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(result["targets"][1]["status"], "blocked", "{status}");
        assert_eq!(
            f.calls()
                .iter()
                .filter(|c| c[0] == "twg" && c[1][1] == "refresh")
                .count(),
            1
        );
    }
}

#[test]
fn human_report_identifies_mode_account_readiness_and_unknown_expiry() {
    let f = Fixture::new(json!({}));
    let out = f
        .command(&["auth", "preflight", "--profile", "code", "--check"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("profile code · check only"), "{stderr}");
    assert!(stderr.contains("[1/2] Checking Bitbucket"), "{stderr}");
    assert!(stderr.contains("[2/2] Checking TWG"), "{stderr}");
    for expected in [
        "Authentication preflight: ready",
        "Bitbucket  valid",
        "Account: office",
        "expiry unknown",
        "2/2 credentials ready",
        "Elapsed ",
    ] {
        assert!(stdout.contains(expected), "missing {expected}: {stdout}");
    }
    let lifetime =
        regex::Regex::new(r"(?m)^  Bitbucket  valid +[0-9]+[hms](?: [0-9]+[hms])* remaining$")
            .unwrap();
    assert!(
        lifetime.is_match(&stdout),
        "missing readable lifetime: {stdout}"
    );
    assert!(stdout.contains("TWG OAuth API access verified"));
    assert!(stdout.contains("twg bb"));
    assert!(stdout.contains("not covered"));
    assert!(!stdout.contains("bitbucket:active"));
    assert!(!text(&out).contains(SECRET));
    assert!(!text(&out).contains('\u{1b}'));
}

#[test]
fn human_blocked_report_labels_recovery_and_does_not_imply_unknown_expiry() {
    let f = Fixture::new(json!({"atlas_valid":false}));
    let out = f
        .command(&["auth", "preflight", "--profile", "logs", "--check"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    for expected in [
        "Authentication preflight: blocked",
        "access not ready",
        "Reason: ",
        "Action: ",
        "0/1 credentials ready",
        "SLAuth",
        "Scope: prod · splunk.paas-inf.net · groups: atlassian-all · MFA",
    ] {
        assert!(stdout.contains(expected), "missing {expected}: {stdout}");
    }
    assert!(!stdout.contains("expiry unknown"));
    assert!(!text(&out).contains(SECRET));
}

#[test]
fn json_check_has_no_human_progress_and_invalid_profile_has_clear_error() {
    let f = Fixture::new(json!({}));
    let out = f
        .command(&[
            "auth",
            "preflight",
            "--profile",
            "code",
            "--check",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(out.stderr.is_empty(), "{}", text(&out));
    let result: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["status"], "completed");
    let out = f
        .command(&["auth", "preflight", "--profile", "unknown"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Authentication preflight: failed"));
    assert!(stdout.contains("Error: Unknown built-in auth profile"));
}

#[test]
fn work_covers_recent_service_dependencies_with_exact_descriptors() {
    let f = Fixture::new(json!({}));
    let r = f.run(&["--check"], 0);
    let labels: Vec<_> = r["targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["target"].as_str().unwrap())
        .collect();
    assert_eq!(
        labels,
        vec![
            "bitbucket:active",
            "twg",
            "slauth:prod:splunk.paas-inf.net:atlassian-all:mfa=true",
            "slauth:prod:risk-mitigation-service::mfa=true",
            "slauth:prod:pollinator::mfa=true",
            "slauth:prod:mlp-config::mfa=false",
            "slauth:prod:phobos::mfa=false",
        ]
    );
    let note = r["targets"][1]["note"].as_str().unwrap();
    assert!(note.contains("twg bb") && note.contains("not covered"));
    let calls = f.calls();
    let atlas: Vec<_> = calls.iter().filter(|c| c[0] == "atlas").collect();
    assert_eq!(atlas.len(), 5);
    for call in atlas {
        let args = call[1].as_array().unwrap();
        let audience = args[3].as_str().unwrap();
        assert_eq!(args[5], "prod");
        assert_eq!(
            args.contains(&json!("--mfa")),
            [
                "splunk.paas-inf.net",
                "risk-mitigation-service",
                "pollinator"
            ]
            .contains(&audience)
        );
        assert_eq!(
            args.contains(&json!("--groups")),
            audience == "splunk.paas-inf.net"
        );
        assert_eq!(args.last().unwrap(), "--cached");
    }
}

#[test]
fn narrow_service_profiles_only_touch_the_selected_audience() {
    for (profile, audience) in [
        ("risk", "risk-mitigation-service"),
        ("checks", "pollinator"),
        ("config", "mlp-config"),
        ("metrics", "phobos"),
    ] {
        let f = Fixture::new(json!({"atlas_valid":false}));
        let r = f.run(&["--profile", profile], 0);
        assert_eq!(r["targets"].as_array().unwrap().len(), 1);
        assert_eq!(r["targets"][0]["status"], "refreshed");
        assert_eq!(f.calls().len(), 3);
        for call in f.calls() {
            assert_eq!(call[0], "atlas");
            assert_eq!(call[1][3], audience);
        }
        let before = f.calls().len();
        f.run(&["--profile", profile, "--check", "--non-interactive"], 0);
        assert_eq!(f.calls().len() - before, 1);
    }
    let f = Fixture::new(json!({}));
    let r = f.run(&["--profile", "services", "--check"], 0);
    assert_eq!(r["targets"].as_array().unwrap().len(), 4);
    assert!(f.calls().iter().all(|c| c[0] == "atlas"));
}

#[test]
fn one_missing_service_blocks_work_and_exec_but_other_audiences_continue() {
    let f = Fixture::new(json!({"atlas_valid_by_audience":{"risk-mitigation-service":false}}));
    let r = f.run(&["--check"], 1);
    assert_eq!(r["targets"][3]["status"], "blocked");
    assert!(r["targets"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 3)
        .all(|(_, t)| t["status"] == "valid"));
    let marker = f.root.path().join("child-must-not-run");
    let out = f
        .command(&[
            "auth",
            "exec",
            "--check",
            "--",
            "/usr/bin/touch",
            marker.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(!marker.exists());
    assert!(!text(&out).contains(SECRET));
    let before = f.calls().len();
    let r = f.run(&[], 0);
    assert_eq!(r["targets"][3]["status"], "refreshed");
    let calls = f.calls();
    let renewals: Vec<_> = calls[before..]
        .iter()
        .filter(|c| c[1].as_array().unwrap().contains(&json!("--force")))
        .collect();
    assert_eq!(renewals.len(), 1);
    assert_eq!(renewals[0][1][3], "risk-mitigation-service");
}

#[test]
fn service_cache_cannot_substitute_a_different_audience_or_start_noninteractive_mfa() {
    for profile in ["risk", "checks", "config", "metrics"] {
        let f = Fixture::new(json!({"aud":"splunk.paas-inf.net"}));
        let r = f.run(&["--profile", profile], 1);
        assert_eq!(r["targets"][0]["status"], "failed");
        assert_eq!(f.calls().len(), 1);
        let f = Fixture::new(json!({"atlas_valid":false}));
        let r = f.run(&["--profile", profile, "--non-interactive"], 1);
        assert_eq!(r["targets"][0]["status"], "blocked");
        assert_eq!(f.calls().len(), 1);
    }
}

#[test]
fn service_progress_identifies_the_audience_before_any_browser_prompt() {
    let f = Fixture::new(json!({}));
    let out = f
        .command(&["auth", "preflight", "--profile", "checks"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let target = stderr
        .find("Renewing SLAuth (prod · pollinator · MFA)")
        .unwrap();
    let prompt = stderr.find("Complete browser sign-in or MFA").unwrap();
    assert!(target < prompt);
    assert!(!text(&out).contains(SECRET));
}
