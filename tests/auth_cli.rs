//! Real workctl CLI, OS process groups, pipes, locks, argv and files. The external
//! provider CLIs are simulated; real account/cache proof lives in auth_live.rs.
use serde_json::{json, Value};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};
const BIN: &str = env!("CARGO_BIN_EXE_workctl");
const SECRET: &str = "AUTH_SECRET_MUST_NEVER_APPEAR";

fn provider_source() -> &'static str {
    static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SOURCE.get_or_init(|| {
        // macOS /usr/bin/python3 is a developer-tool launcher. Resolve it once
        // before any auth deadline starts, rather than launching it per probe.
        let output = Command::new("/usr/bin/python3")
            .args(["-c", "import sys; print(sys.executable)"])
            .output()
            .expect("resolve provider simulator interpreter");
        assert!(
            output.status.success(),
            "Python 3 is required for auth CLI fixtures"
        );
        let python = String::from_utf8(output.stdout).unwrap();
        let python = python.trim();
        assert!(
            std::path::Path::new(python).is_absolute() && !python.chars().any(char::is_whitespace),
            "simulator needs an absolute interpreter path without whitespace"
        );
        let (_, body) = include_str!("fixtures/auth_provider.py")
            .split_once('\n')
            .unwrap();
        format!("#!{python}\n{body}")
    })
}

// Bound independent fixture groups, not concurrency inside the system under
// test. Each group may now launch several providers (and two workctl processes).
// Unbounded test-harness parallelism would measure host scheduling delays against
// short credential lifetimes instead of the intended auth contracts.
static FIXTURE_SLOTS: (std::sync::Mutex<usize>, std::sync::Condvar) =
    (std::sync::Mutex::new(0), std::sync::Condvar::new());
struct FixtureSlot;
impl FixtureSlot {
    fn acquire() -> Self {
        let (mutex, ready) = &FIXTURE_SLOTS;
        let mut active = mutex.lock().unwrap();
        while *active >= 4 {
            active = ready.wait(active).unwrap();
        }
        *active += 1;
        Self
    }
}
impl Drop for FixtureSlot {
    fn drop(&mut self) {
        let (mutex, ready) = &FIXTURE_SLOTS;
        *mutex.lock().unwrap() -= 1;
        ready.notify_one();
    }
}
struct Fixture {
    root: tempfile::TempDir,
    _slot: FixtureSlot,
}
impl Fixture {
    fn new(state: Value) -> Self {
        let slot = FixtureSlot::acquire();
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("bin")).unwrap();
        let script = root.path().join("provider.py");
        fs::write(&script, provider_source()).unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        for name in ["bb", "twg", "atlas", "klist", "kinit"] {
            symlink(&script, root.path().join("bin").join(name)).unwrap();
        }
        let f = Self { root, _slot: slot };
        // Prepare cold executable launches before testing provider deadlines.
        let startup = f.command(&["--help"]).output().unwrap();
        assert!(startup.status.success(), "{}", text(&startup));
        f.state(json!({}));
        let startup = Command::new(f.root.path().join("bin/bb"))
            .args(["auth", "status"])
            .env("AUTH_FIXTURE", f.root.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        assert!(startup.status.success(), "{}", text(&startup));
        for name in [
            "calls.jsonl",
            "started-bb-auth-status",
            "finished-bb-auth-status",
        ] {
            fs::remove_file(f.root.path().join(name)).unwrap();
        }
        f.state(state);
        f
    }
    fn state(&self, state: Value) {
        for provider in ["bb", "twg", "atlas", "klist", "kinit"] {
            let _ = fs::remove_file(self.root.path().join(format!("{provider}-state.json")));
        }
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
    let end = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(Instant::now() < end, "fixture did not start: {path:?}");
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
    assert_eq!(f.calls().len(), 18);
    assert!(f.calls().iter().all(|c| c[1][1] == "status"
        || c[1][0] == "workspace"
        || c[1][0] == "doctor"
        || c[1].as_array().unwrap().contains(&json!("--cached"))));
}
#[test]
fn all_missing_renewable_credentials_are_refreshed_once_and_normal_cache_is_rechecked() {
    let f = Fixture::new(json!({"bb_valid":false,"twg_valid":false,"atlas_valid":false}));
    let r = f.run(&[], 0);
    assert!(r["targets"]
        .as_array()
        .unwrap()
        .iter()
        .all(|v| v["status"] == "refreshed"
            || (v["target"] == "twg-bitbucket" && v["status"] == "valid")));
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
    assert_eq!(f.calls().len() - before, 9);
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
    assert_eq!(f.calls().len(), 8);
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
        assert_eq!(f.calls().len(), 8);
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
                "10",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let started = f.root.path().join("started");
        let startup_deadline = Instant::now() + Duration::from_secs(10);
        while !started.exists() {
            let stopped = child.try_wait().unwrap().is_some();
            if stopped || Instant::now() >= startup_deadline {
                if !stopped {
                    unsafe {
                        libc::kill(child.id() as i32, libc::SIGTERM);
                    }
                }
                let output = child.wait_with_output().unwrap();
                panic!("Provider fixture did not start: {}", text(&output));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
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
            assert!(began.elapsed() < Duration::from_secs(12));
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
    let f = Fixture::new(json!({"bb_seconds":8,"hold_keys":["twg-doctor"]}));
    let child = f.start(&["--profile", "code", "--check", "--min-valid-for", "0"]);
    f.started("twg-doctor");
    f.finished("bb-workspace-list");
    // Let an already verified credential expire while another provider waits.
    std::thread::sleep(Duration::from_secs(9));
    f.release("twg-doctor");
    let r = child.finish(1);
    assert_eq!(r["targets"][0]["status"], "blocked", "{r}");
    assert_eq!(r["targets"][0]["remaining_seconds"], 0);
    assert!(r["targets"][0]["reason"].as_str().unwrap().contains("aged"));
}

#[test]
fn bitbucket_expiry_during_api_verification_blocks_preflight_and_exec() {
    for exec in [false, true] {
        let f = Fixture::new(json!({"bb_seconds":8,"bb_api_delay":9}));
        let profile = f.profile(json!([{"provider":"bitbucket"}]));
        let marker = f.root.path().join("child-started");
        let mut command = f.command(&["auth", if exec { "exec" } else { "preflight" }]);
        command.args(["--check", "--config", &profile, "--min-valid-for", "0"]);
        if exec {
            command.args(["--", "/usr/bin/touch"]).arg(&marker);
        } else {
            command.arg("--json");
        }
        let out = command.output().unwrap();
        assert!(
            !marker.exists(),
            "expired credentials must not launch the child"
        );
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert_eq!(f.calls().len(), 2, "check-only must not renew or retry");
        assert_eq!(f.calls()[1][1], json!(["workspace", "list", "--json"]));
        if !exec {
            let result: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(result["status"], "blocked");
            assert_eq!(result["targets"][0]["status"], "blocked");
        }
    }
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
fn default_preflight_forces_every_renewable_provider_once_on_every_invocation() {
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
            .all(|v| v["status"] == "refreshed"
                || (v["target"] == "twg-bitbucket" && v["status"] == "valid")));
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
        .filter(|v| v["target"] != "twg-bitbucket")
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
    assert!(stderr.contains("[1/3] Checking Bitbucket"), "{stderr}");
    assert!(stderr.contains("[2/3] Checking TWG"), "{stderr}");
    for expected in [
        "Authentication preflight: ready",
        "Bitbucket  valid",
        "Account: office",
        "expiry unknown",
        "3/3 credentials ready",
        "Elapsed ",
    ] {
        assert!(stdout.contains(expected), "missing {expected}: {stdout}");
    }
    let lifetime =
        regex::Regex::new(r"(?m)^  Bitbucket +valid +[0-9]+[hms](?: [0-9]+[hms])* remaining$")
            .unwrap();
    assert!(
        lifetime.is_match(&stdout),
        "missing readable lifetime: {stdout}"
    );
    assert!(stdout.contains("TWG OAuth API access verified"));
    assert!(stdout.contains("TWG Bitbucket API access verified"));
    assert!(stdout.contains("never automatically rotated"));
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
            "twg-bitbucket",
        ]
    );
    let note = r["targets"][1]["note"].as_str().unwrap();
    assert!(note.contains("TWG Bitbucket") && note.contains("separately"));
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

// Barrier-driven concurrency contracts: no elapsed-time speedup assertions.
struct RunningAuth(std::process::Child);
impl RunningAuth {
    fn finish(mut self, expected: i32) -> Value {
        let deadline = Instant::now() + Duration::from_secs(15);
        while self.0.try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "auth workers did not finish");
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut stdout = String::new();
        let mut stderr = String::new();
        use std::io::Read;
        self.0
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut stdout)
            .unwrap();
        self.0
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert_eq!(
            self.0.wait().unwrap().code(),
            Some(expected),
            "{stderr}\n{stdout}"
        );
        assert!(!stdout.contains(SECRET) && !stderr.contains(SECRET));
        serde_json::from_str(&stdout).unwrap()
    }
}
impl Drop for RunningAuth {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            unsafe {
                libc::kill(self.0.id() as i32, libc::SIGTERM);
            }
            let _ = self.0.wait();
        }
    }
}
impl Fixture {
    fn start(&self, extra: &[&str]) -> RunningAuth {
        RunningAuth(
            self.command(&["auth", "preflight", "--json", "--timeout", "30"])
                .args(extra)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        )
    }
    fn started(&self, key: &str) {
        wait_file(&self.root.path().join(format!("started-{key}")));
    }
    fn finished(&self, key: &str) {
        wait_file(&self.root.path().join(format!("finished-{key}")));
    }
    fn release(&self, key: &str) {
        fs::write(self.root.path().join(format!("release-{key}")), "").unwrap();
    }
    fn has_started(&self, key: &str) -> bool {
        self.root.path().join(format!("started-{key}")).exists()
    }
}

#[test]
fn provider_checks_overlap_and_completion_order_does_not_change_report_order() {
    let f = Fixture::new(json!({"assert_serial":true,
        "hold_keys":["bb-auth-status", "twg-doctor", "atlas-cache-splunk.paas-inf.net"]}));
    let child = f.start(&["--check"]);
    // Current sequential implementation cannot reach all three barriers.
    for key in [
        "bb-auth-status",
        "twg-doctor",
        "atlas-cache-splunk.paas-inf.net",
    ] {
        f.started(key);
    }
    f.release("twg-doctor");
    f.finished("twg-doctor");
    f.release("atlas-cache-splunk.paas-inf.net");
    f.finished("atlas-cache-phobos");
    assert!(!f.has_started("bb-workspace-list"));
    f.release("bb-auth-status");
    let result = child.finish(0);
    let labels: Vec<_> = result["targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["target"].as_str().unwrap())
        .collect();
    assert_eq!(labels.len(), 8);
    assert_eq!(&labels[..2], &["bitbucket:active", "twg"]);
    assert!(labels[2].contains("splunk.paas-inf.net"));
    assert!(labels[6].contains("phobos"));
    assert!(!f.root.path().join("provider-overlap").exists());
}

#[test]
fn custom_targets_share_one_worker_per_provider() {
    let f = Fixture::new(
        json!({"assert_serial":true,"hold_keys":["bb-auth-status", "atlas-cache-first"]}),
    );
    let config = f.profile(json!([
        {"provider":"slauth","audience":"first"},
        {"provider":"bitbucket","account":"office"},
        {"provider":"slauth","audience":"second"},
        {"provider":"bitbucket","account":"other"}
    ]));
    let child = f.start(&["--check", "--config", &config]);
    f.started("bb-auth-status");
    f.started("atlas-cache-first");
    assert!(!f.has_started("atlas-cache-second"));
    assert_eq!(f.calls().iter().filter(|v| v[0] == "bb").count(), 1);
    f.release("bb-auth-status");
    f.release("atlas-cache-first");
    let result = child.finish(0);
    assert_eq!(result["targets"][3]["target"], "bitbucket:other");
    assert!(!f.root.path().join("provider-overlap").exists());
}

#[test]
fn interactive_commands_are_exclusive_while_background_work_continues() {
    let f = Fixture::new(json!({"assert_interaction":true,"assert_serial":true,
        "hold_keys":["bb-auth-login", "atlas-force-service", "twg-doctor"]}));
    let config = f.profile(json!([
        {"provider":"bitbucket"}, {"provider":"twg"}, {"provider":"slauth","audience":"service"}
    ]));
    let child = f.start(&["--config", &config]);
    f.started("twg-doctor");
    let end = Instant::now() + Duration::from_secs(5);
    while !f.has_started("bb-auth-login") && !f.has_started("atlas-force-service") {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(10));
    }
    let (first, second) = if f.has_started("bb-auth-login") {
        ("bb-auth-login", "atlas-force-service")
    } else {
        ("atlas-force-service", "bb-auth-login")
    };
    f.finished("bb-auth-status");
    f.finished("atlas-cache-service");
    f.release("twg-doctor");
    f.finished("twg-auth-refresh");
    assert!(!f.root.path().join("interaction-overlap").exists());
    assert!(!f.has_started(second));
    f.release(first);
    f.started(second);
    f.release(second);
    let result = child.finish(0);
    assert!(result["targets"]
        .as_array()
        .unwrap()
        .iter()
        .all(|v| v["status"] == "refreshed"
            || (v["target"] == "twg-bitbucket" && v["status"] == "valid")));
    assert!(!f.root.path().join("interaction-overlap").exists());
}

#[test]
fn queued_login_receives_its_full_command_deadline() {
    let f = Fixture::new(json!({"assert_interaction":true,
        "hold_keys":["bb-auth-status", "atlas-force-service", "bb-auth-login"]}));
    let config = f.profile(json!([
        {"provider":"slauth","audience":"service"}, {"provider":"bitbucket"}
    ]));
    let child = f.start(&[
        "--config",
        &config,
        "--timeout",
        "2",
        "--login-timeout",
        "5",
    ]);
    f.started("bb-auth-status");
    f.started("atlas-force-service");
    f.release("bb-auth-status");
    f.finished("bb-workspace-list");
    std::thread::sleep(Duration::from_secs(3));
    f.release("atlas-force-service");
    f.started("bb-auth-login");
    std::thread::sleep(Duration::from_secs(3));
    f.release("bb-auth-login");
    let result = child.finish(0);
    assert_eq!(result["status"], "completed");
    assert_eq!(result["targets"].as_array().unwrap().len(), 2);
    for target in result["targets"].as_array().unwrap() {
        assert_eq!(target["status"], "refreshed", "{result}");
    }
    assert!(!f.root.path().join("interaction-overlap").exists());
}

#[test]
fn provider_timeout_preserves_results_and_continues_other_provider_queues() {
    let f = Fixture::new(json!({"hang_keys":["twg-doctor"],
        "hold_keys":["atlas-cache-first", "atlas-cache-second"]}));
    let config = f.profile(json!([
        {"provider":"twg"},
        {"provider":"slauth","audience":"first"},
        {"provider":"slauth","audience":"second"},
        {"provider":"slauth","audience":"third"},
        {"provider":"bitbucket"}
    ]));
    let child = f.start(&["--check", "--config", &config, "--timeout", "5"]);
    f.started("twg-doctor");
    f.started("atlas-cache-first");
    std::thread::sleep(Duration::from_secs(2));
    f.release("atlas-cache-first");
    f.started("atlas-cache-second");
    let descendant: i32 = fs::read_to_string(f.root.path().join("descendant-twg-doctor"))
        .unwrap()
        .parse()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    while alive(descendant) {
        assert!(
            Instant::now() < deadline,
            "timed-out provider descendant is still active"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    f.release("atlas-cache-second");
    let result = child.finish(1);
    assert_eq!(result["status"], "blocked", "{result}");
    let targets = result["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 5);
    assert_eq!(targets[0]["target"], "twg");
    assert_eq!(targets[0]["status"], "failed");
    assert!(targets[0]["reason"].as_str().unwrap().contains("deadline"));
    for (target, label) in targets[1..].iter().zip([
        "slauth:prod:first::mfa=false",
        "slauth:prod:second::mfa=false",
        "slauth:prod:third::mfa=false",
        "bitbucket:active",
    ]) {
        assert_eq!(target["target"], label);
        assert_eq!(target["status"], "valid", "{result}");
    }
    assert!(f.has_started("atlas-cache-third"));
}

#[test]
fn cancellation_cleans_multiple_workers_and_does_not_launch_queued_interaction() {
    let f = Fixture::new(json!({"assert_interaction":true,
        "hold_keys":["bb-auth-login", "atlas-force-service"],
        "hang_keys":["twg-doctor", "bb-auth-login", "atlas-force-service"]}));
    let config = f.profile(json!([
        {"provider":"bitbucket"}, {"provider":"twg"}, {"provider":"slauth","audience":"service"}
    ]));
    let child = f.start(&["--config", &config]);
    f.started("twg-doctor");
    f.finished("bb-auth-status");
    f.finished("atlas-cache-service");
    let end = Instant::now() + Duration::from_secs(5);
    while !f.has_started("bb-auth-login") && !f.has_started("atlas-force-service") {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(10));
    }
    unsafe {
        libc::kill(child.0.id() as i32, libc::SIGTERM);
    }
    child.finish(130);
    assert_ne!(
        f.has_started("bb-auth-login"),
        f.has_started("atlas-force-service")
    );
    for entry in fs::read_dir(f.root.path()).unwrap().flatten() {
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with("descendant-")
        {
            let pid = fs::read_to_string(entry.path()).unwrap().parse().unwrap();
            assert!(!alive(pid));
        }
    }
    f.state(json!({}));
    f.run(&["--check"], 0); // Every worker stopped and the coordination lock released.
}

#[test]
fn coordination_timeout_is_independent_of_provider_command_timeout() {
    let f = Fixture::new(json!({"hold_keys":["atlas-cache-splunk.paas-inf.net"]}));
    let first = f.start(&["--profile", "logs", "--check"]);
    f.started("atlas-cache-splunk.paas-inf.net");
    let result = f
        .start(&[
            "--profile",
            "logs",
            "--check",
            "--coordination-timeout",
            "1",
        ])
        .finish(1);
    assert!(result["error"]
        .as_str()
        .unwrap()
        .contains("coordination deadline"));
    assert_eq!(f.calls().len(), 1);
    let second = f.start(&[
        "--profile",
        "logs",
        "--check",
        "--timeout",
        "1",
        "--coordination-timeout",
        "5",
    ]);
    // Exceed the provider timeout while the second invocation is queued.
    std::thread::sleep(Duration::from_millis(1200));
    f.release("atlas-cache-splunk.paas-inf.net");
    first.finish(0);
    second.finish(0);
}

#[test]
fn default_work_blocks_missing_twg_bitbucket_even_when_oauth_and_bb_work() {
    let f = Fixture::new(json!({"twg_bb_valid": false}));
    let r = f.run(&["--check"], 1);
    let targets = r["targets"].as_array().unwrap();
    let token = targets
        .iter()
        .find(|t| t["target"] == "twg-bitbucket")
        .unwrap();
    assert_eq!(token["status"], "blocked");
    assert!(token["action"]
        .as_str()
        .unwrap()
        .contains("twg setup bitbucket --force"));
    assert_eq!(targets[0]["status"], "valid");
    assert_eq!(targets[1]["status"], "valid");
    assert!(f.calls().iter().all(|c| c[1][0] != "setup"));
}

#[test]
fn missing_twg_bitbucket_prevents_wrapped_command_without_renewing_other_auth() {
    let f = Fixture::new(json!({"twg_bb_valid": false}));
    let marker = f.root.path().join("must-not-run");
    let out = f
        .command(&[
            "auth",
            "exec",
            "--profile",
            "code",
            "--reuse-valid",
            "--",
            "/usr/bin/touch",
            marker.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(!marker.exists());
    assert!(!text(&out).contains(SECRET));
    assert!(f
        .calls()
        .iter()
        .all(|c| c[1][0] != "setup" && c[1][1] != "refresh" && c[1][1] != "login"));
}

#[test]
fn twg_bitbucket_checks_only_its_own_evidence_and_never_rotates_a_valid_token() {
    for args in [vec![], vec!["--check"], vec!["--non-interactive"]] {
        let f = Fixture::new(json!({"twg_valid":false}));
        let mut command = f.command(&["auth", "preflight", "--profile", "twg-bitbucket", "--json"]);
        let out = command.args(args).output().unwrap();
        assert!(out.status.success(), "{}", text(&out));
        let r: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(r["targets"][0]["status"], "valid");
        assert!(r["targets"][0]["note"]
            .as_str()
            .unwrap()
            .contains("never automatically rotated"));
        assert_eq!(f.calls().len(), 1);
        assert_eq!(f.calls()[0][1][0], "doctor");
    }
}

#[test]
fn twg_bitbucket_failures_are_classified_without_refresh_or_secret_output() {
    for (response, expected) in [
        (
            json!({"attempted":false,"status":"missing","ok":false}),
            "blocked",
        ),
        (
            json!({"attempted":true,"status":"invalid-or-expired","ok":false}),
            "blocked",
        ),
        (
            json!({"attempted":true,"status":"forbidden","ok":false}),
            "failed",
        ),
        (
            json!({"attempted":true,"status":"error","ok":false,"message":SECRET}),
            "failed",
        ),
        (
            json!({"attempted":true,"status":"valid","ok":false}),
            "failed",
        ),
        (json!({"ok":true}), "failed"),
        (json!({}), "failed"),
        (
            json!({"attempted":false,"status":"valid","ok":true}),
            "failed",
        ),
    ] {
        let f = Fixture::new(json!({"twg_bb_response":response}));
        let r = f.run(&["--profile", "twg-bitbucket"], 1);
        assert_eq!(r["targets"][0]["status"], expected);
        assert_eq!(f.calls().len(), 1);
    }
}

#[test]
fn kerberos_checks_principal_and_renews_only_once_without_interactive_login() {
    let f = Fixture::new(json!({}));
    let config = f.profile(json!([{"provider":"kerberos","principal":"fixture@EXAMPLE.COM"}]));
    let r = f.run(&["--config", &config, "--check"], 0);
    assert_eq!(r["targets"][0]["account"], "fixture@EXAMPLE.COM");
    assert_eq!(r["targets"][0]["remaining_seconds"], Value::Null);
    assert_eq!(f.calls().len(), 2);
    f.state(json!({"kerberos_valid":false}));
    let before = f.calls().len();
    f.run(&["--config", &config, "--check"], 1);
    assert_eq!(f.calls().len() - before, 1);
    let r = f.run(&["--config", &config, "--non-interactive"], 0);
    assert_eq!(r["targets"][0]["status"], "refreshed");
    assert_eq!(f.calls().iter().filter(|c| c[0] == "kinit").count(), 1);
    assert!(f
        .calls()
        .iter()
        .filter(|c| c[0] == "kinit")
        .all(|c| c[1] == json!(["-R"])));
}

#[test]
fn kerberos_wrong_identity_and_provider_failures_never_trigger_renewal() {
    for state in [
        json!({"kerberos_principal":"someone@EXAMPLE.COM"}),
        json!({"mode":"malformed"}),
        json!({"mode":"permission"}),
        json!({"mode":"outage"}),
    ] {
        let f = Fixture::new(state);
        let config = f.profile(json!([{"provider":"kerberos","principal":"fixture@EXAMPLE.COM"}]));
        let r = f.run(&["--config", &config], 1);
        assert_eq!(r["targets"][0]["status"], "failed");
        assert!(f.calls().iter().all(|c| c[0] == "klist"));
    }
    let f = Fixture::new(json!({"kerberos_valid":false,"refresh_fails":true}));
    let r = f.run(&["--profile", "runtime", "--non-interactive"], 1);
    assert_eq!(r["targets"][0]["status"], "blocked");
    assert_eq!(f.calls().iter().filter(|c| c[0] == "kinit").count(), 1);
}

#[test]
fn included_profiles_add_requirements_deduplicate_and_validate_before_auth() {
    let f = Fixture::new(json!({}));
    let r = f.run(
        &[
            "--profile",
            "code",
            "--include-profile",
            "runtime",
            "--include-profile",
            "twg",
            "--check",
        ],
        0,
    );
    assert_eq!(r["targets"].as_array().unwrap().len(), 4);
    assert_eq!(r["targets"][3]["target"], "kerberos:default");
    assert_eq!(f.calls().iter().filter(|c| c[0] == "twg").count(), 2);
    let f = Fixture::new(json!({"kerberos_valid":false}));
    let r = f.run(
        &[
            "--profile",
            "code",
            "--include-profile",
            "runtime",
            "--check",
        ],
        1,
    );
    assert_eq!(r["targets"][3]["status"], "blocked");
    let f = Fixture::new(json!({}));
    f.run(
        &[
            "--profile",
            "code",
            "--include-profile",
            "unknown",
            "--check",
        ],
        1,
    );
    assert!(f.calls().is_empty());
}

#[test]
fn expired_kerberos_for_another_principal_cannot_be_renewed_as_the_requested_identity() {
    let f =
        Fixture::new(json!({"kerberos_valid":false,"kerberos_principal":"someone@EXAMPLE.COM"}));
    let config = f.profile(json!([{"provider":"kerberos","principal":"fixture@EXAMPLE.COM"}]));
    let r = f.run(&["--config", &config, "--non-interactive"], 1);
    assert_eq!(r["targets"][0]["status"], "failed");
    assert!(f.calls().iter().all(|c| c[0] == "klist"));
}

#[test]
fn included_custom_profiles_preserve_scope_and_block_child_on_missing_runtime_auth() {
    let f = Fixture::new(json!({"kerberos_valid":false}));
    let path = f.root.path().join("combined.json");
    fs::write(
        &path,
        json!({"schema_version":1,"profiles":{
            "work":[{"provider":"twg_bitbucket"}],
            "runtime":[{"provider":"kerberos","principal":"fixture@EXAMPLE.COM"}]
        }})
        .to_string(),
    )
    .unwrap();
    let marker = f.root.path().join("must-not-run");
    let out = f
        .command(&[
            "auth",
            "exec",
            "--config",
            path.to_str().unwrap(),
            "--include-profile",
            "runtime",
            "--check",
            "--",
            "/usr/bin/touch",
            marker.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(!marker.exists());
    assert!(f.calls().iter().all(|c| c[0] == "twg" || c[0] == "klist"));
    assert!(!text(&out).contains(SECRET));
    for principal in ["unqualified", "user@", "@REALM", "-user@REALM"] {
        let f = Fixture::new(json!({}));
        let config = f.profile(json!([{"provider":"kerberos","principal":principal}]));
        f.run(&["--config", &config], 1);
        assert!(f.calls().is_empty());
    }
}

#[test]
fn twg_bitbucket_error_cannot_trigger_unrelated_oauth_renewal() {
    for args in [vec!["--reuse-valid"], vec!["--check"]] {
        let f = Fixture::new(json!({
            "twg_valid": false,
            "twg_token_present": true,
            "twg_bb_response": {"attempted":true,"ok":false,
                "status":"invalid-or-expired","message":"Bitbucket token has expired"}
        }));
        let mut command = f.command(&["auth", "preflight", "--profile", "twg", "--json"]);
        let out = command.args(args).output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        let r: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(r["targets"][0]["status"], "failed");
        assert_eq!(
            f.calls().len(),
            1,
            "must not renew OAuth for a Bitbucket error"
        );
    }
}

#[test]
fn manual_bitbucket_report_preserves_evidence_without_claiming_renewal() {
    for (provider_status, attempted, reason) in [
        ("missing", false, "not configured"),
        ("invalid-or-expired", true, "rejected"),
    ] {
        let f = Fixture::new(json!({"twg_bb_response":{
            "status":provider_status,"attempted":attempted,"ok":false,"message":SECRET
        }}));
        let r = f.run(&["--profile", "twg-bitbucket"], 1);
        let target = &r["targets"][0];
        assert_eq!(target["provider_status"], provider_status);
        assert!(target["reason"].as_str().unwrap().contains(reason));
        assert!(target["note"]
            .as_str()
            .unwrap()
            .contains("never automatically rotated"));
        let out = f
            .command(&["auth", "preflight", "--profile", "twg-bitbucket"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1));
        let report = text(&out);
        assert!(report.contains("Checking TWG Bitbucket"), "{report}");
        assert!(!report.contains("Renewing TWG Bitbucket"), "{report}");
        assert!(!report.contains("renew all credentials"), "{report}");
        assert!(!report.contains(SECRET));
        assert!(f.calls().iter().all(|c| c[1][0] == "doctor"));
    }
}

#[test]
fn twg_oauth_expiry_is_still_classified_as_auth_when_a_token_is_present() {
    let f = Fixture::new(json!({
        "twg_token_present": true,
        "twg_connectivity_response": {"attempted":true,"ok":false,"message":"OAuth token has expired"}
    }));
    let r = f.run(&["--profile", "twg", "--check"], 1);
    assert_eq!(r["targets"][0]["status"], "blocked");
    assert_eq!(f.calls().len(), 1);
}
