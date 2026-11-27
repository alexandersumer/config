//! Public `workctl docker image clean` contracts against a scripted Docker CLI.
use serde_json::{json, Value};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_workctl");

/// Replays Docker's documented report formats and records every invocation.
const DOCKER: &str = r#"#!/bin/sh
state=$FAKE_DOCKER
if [ "$1" = --host ]; then
  [ "$2" = "unix://$state/desktop.sock" ] || exit 61
  [ -z "${DOCKER_HOST+x}${DOCKER_CONTEXT+x}${DOCKER_TLS_VERIFY+x}${DOCKER_CERT_PATH+x}" ] || exit 62
  printf '%s\n' "$2" >>"$state/sockets"
  shift 2
fi
printf '%s\n' "$*" >>"$state/calls"
if [ -e "$state/unavailable" ]; then
  echo 'Cannot connect to the Docker daemon' >&2
  exit 1
fi
case "$1 $2 $3" in
'system df --format')
  calls=$(grep -c '^system df --format' "$state/calls")
  size=$(sed -n "${calls}p" "$state/usage")
  [ -n "$size" ] || size=$(tail -n 1 "$state/usage")
  if [ "$size" = fail ]; then
    echo 'daemon went away' >&2
    exit 1
  fi
  printf '{"Type":"Images","Size":"%s"}\n{"Type":"Containers","Size":"0B"}\n' "$size"
  ;;
'system df --verbose') cat "$state/report.json" ;;
'image inspect '*)
  cat "$state/inspect.json"
  [ ! -e "$state/vanished" ]
  ;;
'image rm --no-prune')
  target=$4
  if grep -qxF "$target" "$state/multi_repository" 2>/dev/null; then
    echo "Error response from daemon: conflict: unable to delete $target (must be forced) - image is referenced in multiple repositories" >&2
    exit 1
  fi
  if grep -qxF "$target" "$state/refused" 2>/dev/null; then
    echo "Error response from daemon: conflict: unable to delete $target" >&2
    exit 1
  fi
  ;;
'image ls --all') [ ! -e "$state/listed" ] || cat "$state/listed" ;;
'container ls --all')
  if grep -qF "${6#ancestor=}" "$state/users" 2>/dev/null; then echo c0ffee; fi
  ;;
*)
  echo "unexpected docker $*" >&2
  exit 64
  ;;
esac
"#;

struct Image {
    id: &'static str,
    tags: &'static [&'static str],
    digests: &'static [&'static str],
    arrived: &'static str,
    unique: &'static str,
    containers: &'static str,
}

const fn image(id: &'static str, arrived: &'static str, unique: &'static str) -> Image {
    Image {
        id,
        tags: &[],
        digests: &[],
        arrived,
        unique,
        containers: "0",
    }
}

struct Fixture {
    temp: tempfile::TempDir,
}

impl Fixture {
    /// `usage` is the image store size Docker reports on each successive measurement.
    fn new(usage: &[&str], images: &[Image]) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let fixture = Self { temp };
        let docker = fixture.state("docker");
        fs::write(&docker, DOCKER).unwrap();
        fs::set_permissions(&docker, fs::Permissions::from_mode(0o755)).unwrap();
        fixture.write("usage", &usage.join("\n"));
        let rows: Vec<_> = images
            .iter()
            .map(|image| json!({"ID": image.id, "UniqueSize": image.unique, "Containers": image.containers, "Size": image.unique, "SharedSize": "0B"}))
            .collect();
        fixture.write(
            "report.json",
            &json!({"Images": rows, "Containers": [], "Volumes": [], "BuildCache": []}).to_string(),
        );
        fixture.inspect(images);
        fixture
    }
    fn inspect(&self, images: &[Image]) {
        let details: Vec<_> = images
            .iter()
            .map(|image| json!({"Id": image.id, "RepoTags": image.tags, "RepoDigests": image.digests, "Metadata": {"LastTagTime": image.arrived}}))
            .collect();
        self.write("inspect.json", &Value::from(details).to_string());
    }
    fn state(&self, name: &str) -> PathBuf {
        self.temp.path().join(name)
    }
    fn write(&self, name: &str, contents: &str) {
        fs::write(self.state(name), contents).unwrap();
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(BIN)
            .args(["docker", "image", "clean"])
            .args(args)
            .env("FAKE_DOCKER", self.temp.path())
            .env("DOCKER_HOST", "unix:///incorrect.sock")
            .env("DOCKER_CONTEXT", "incorrect")
            .env("DOCKER_TLS_VERIFY", "1")
            .env("DOCKER_CERT_PATH", "/incorrect")
            .env("WORKCTL_STATE_DIR", self.state("state"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.temp.path().display()),
            )
            .env("NO_COLOR", "1")
            .env("TERM", "dumb")
            .output()
            .unwrap()
    }
    fn result(&self, args: &[&str], code: i32) -> Value {
        let out = self.run(&[args, &["--json"]].concat());
        assert_eq!(out.status.code(), Some(code), "{}", text(&out));
        serde_json::from_slice(&out.stdout).expect("stdout is one JSON document")
    }
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.state("calls"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
    fn removals(&self) -> Vec<String> {
        self.calls()
            .into_iter()
            .filter_map(|call| call.strip_prefix("image rm --no-prune ").map(str::to_owned))
            .collect()
    }
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn statuses(result: &Value) -> Vec<(String, String)> {
    result["images"]
        .as_array()
        .unwrap()
        .iter()
        .map(|image| {
            (
                image["id"].as_str().unwrap().to_owned(),
                image["status"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// A hundred gigabytes against an eighty gigabyte budget leaves twenty to find.
fn crowded() -> Vec<Image> {
    vec![
        image("sha256:newer", "2020-06-01T00:00:00Z", "9GB"),
        Image {
            containers: "2",
            ..image("sha256:used", "2019-01-01T00:00:00Z", "30GB")
        },
        image("sha256:unknown", "0001-01-01T00:00:00Z", "40GB"),
        image("sha256:oldest", "2020-01-01T00:00:00.123456789Z", "12GB"),
        image("sha256:recent", "2999-01-01T00:00:00Z", "40GB"),
        image("sha256:spare", "2021-01-01T00:00:00Z", "50GB"),
    ]
}

#[test]
fn store_within_budget_needs_no_inventory_and_removes_nothing() {
    let fixture = Fixture::new(&["79.9GB"], &crowded());
    let result = fixture.result(&["--max-used-space", "80GB", "--apply"], 0);
    assert_eq!(result["operation"], "docker.image.clean");
    assert!(result["started_unix_seconds"].as_u64().unwrap() > 1_700_000_000);
    assert_eq!(result["status"], "completed");
    assert_eq!(result["used_bytes"], 79_900_000_000u64);
    assert_eq!(result["within_budget"], true);
    assert_eq!(result["summary"]["selected"], 0);
    assert_eq!(fixture.calls(), ["system df --format {{json .}}"]);
}

#[test]
fn preview_selects_the_oldest_unused_arrivals_that_cover_the_excess() {
    let fixture = Fixture::new(&["100GB"], &crowded());
    let args = ["--max-used-space", "80GB", "--older-than", "2h"];
    let result = fixture.result(&args, 0);
    assert_eq!(
        statuses(&result),
        [
            ("sha256:oldest".into(), "selected".into()),
            ("sha256:newer".into(), "selected".into())
        ]
    );
    assert_eq!(
        result["summary"],
        json!({"images": 6, "in_use": 1, "recent": 1, "unknown_arrival": 1, "selected": 2,
            "removed": 0, "skipped": 0, "failed": 0, "estimated_bytes": 21_000_000_000u64})
    );
    assert_eq!(result["apply"], false);
    assert_eq!(result["older_than_seconds"], 7200);
    assert_eq!(result["used_bytes_after"], Value::Null);
    assert_eq!(result["within_budget"], true);
    assert!(fixture.removals().is_empty());

    let out = fixture.run(&args);
    let report = text(&out);
    assert_eq!(out.status.code(), Some(0), "{report}");
    for expected in [
        "Docker image cleanup preview",
        "Selected       2 (about 21.0 GB)",
        "selected 2020-01-01",
        "Next: workctl docker image clean --max-used-space 80000000000B --older-than 7200s --apply",
        "Completed",
    ] {
        assert!(report.contains(expected), "{expected}: {report}");
    }
    assert!(fixture.removals().is_empty());
}

#[test]
fn apply_removes_by_id_without_force_and_measures_until_the_store_fits() {
    let fixture = Fixture::new(&["100GB", "85GB", "78GB"], &crowded());
    let result = fixture.result(&["--max-used-space", "80GB", "--apply"], 0);
    // The first batch falls short of its estimate, so the next oldest follows.
    assert_eq!(
        fixture.removals(),
        ["sha256:oldest", "sha256:newer", "sha256:spare"]
    );
    assert!(fixture
        .calls()
        .iter()
        .all(|call| !call.split(' ').any(|arg| arg == "-f" || arg == "--force")));
    assert_eq!(result["used_bytes_after"], 78_000_000_000u64);
    assert_eq!(result["within_budget"], true);
    assert_eq!(result["summary"]["removed"], 3);
    assert_eq!(result["status"], "completed");
}

#[test]
fn a_store_held_by_protected_images_reports_the_shortfall_as_success() {
    let fixture = Fixture::new(&["100GB", "91GB"], &crowded());
    let args = ["--max-used-space", "50GB", "--older-than", "2h", "--apply"];
    let out = fixture.run(&args);
    let report = text(&out);
    assert_eq!(out.status.code(), Some(0), "{report}");
    assert_eq!(
        fixture.removals(),
        ["sha256:oldest", "sha256:newer", "sha256:spare"]
    );
    for expected in [
        "Removed        3",
        "Used after     91.0 GB",
        "Over budget    about 41.0 GB",
    ] {
        assert!(report.contains(expected), "{expected}: {report}");
    }
}

#[test]
fn images_a_container_now_uses_are_kept_while_other_refusals_fail() {
    let images = [
        image("sha256:claimed", "2020-01-01T00:00:00Z", "12GB"),
        image("sha256:plain", "2020-06-01T00:00:00Z", "9GB"),
    ];
    let fixture = Fixture::new(&["100GB"], &images);
    fixture.write("refused", "sha256:claimed\n");
    fixture.write("listed", "sha256:claimed\n");
    fixture.write("users", "sha256:claimed\n");
    let result = fixture.result(&["--max-used-space", "80GB", "--apply"], 0);
    assert_eq!(
        statuses(&result),
        [
            ("sha256:claimed".into(), "in_use".into()),
            ("sha256:plain".into(), "removed".into())
        ]
    );
    assert_eq!(result["summary"]["skipped"], 1);

    let fixture = Fixture::new(&["100GB"], &images);
    fixture.write("refused", "sha256:plain\n");
    fixture.write("listed", "sha256:plain\n");
    let result = fixture.result(&["--max-used-space", "80GB", "--apply"], 1);
    assert_eq!(result["status"], "failed");
    assert_eq!(result["images"][1]["status"], "failed");
    assert!(result["images"][1]["error"]
        .as_str()
        .unwrap()
        .contains("unable to delete sha256:plain"));
}

#[test]
fn shared_tags_are_preserved_and_every_docker_call_uses_the_pinned_socket() {
    let shared = Image {
        tags: &["app:1", "registry.example/team/app:1"],
        digests: &["app@sha256:digest"],
        ..image("sha256:shared", "2020-01-01T00:00:00Z", "30GB")
    };
    let fixture = Fixture::new(&["100GB"], &[shared]);
    fixture.write("multi_repository", "sha256:shared\n");
    let socket = fixture.state("desktop.sock");
    let result = fixture.result(
        &[
            "--max-used-space",
            "80GB",
            "--apply",
            "--docker-socket",
            socket.to_str().unwrap(),
        ],
        0,
    );
    assert_eq!(fixture.removals(), ["sha256:shared"]);
    let calls = fixture.calls();
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.starts_with("image rm "))
            .count(),
        1
    );
    let endpoints = fs::read_to_string(fixture.state("sockets")).unwrap();
    assert_eq!(endpoints.lines().count(), calls.len());
    assert_eq!(result["images"][0]["status"], "protected");
    assert_eq!(result["summary"]["skipped"], 1);
    assert_eq!(result["within_budget"], false);
    assert_eq!(
        result["images"][0]["references"],
        json!(["app:1", "registry.example/team/app:1", "app@sha256:digest"])
    );
}

#[test]
fn images_that_vanish_during_inventory_are_ignored() {
    let images = crowded();
    let fixture = Fixture::new(&["100GB"], &images);
    fixture.inspect(&images[..1]);
    fixture.write("vanished", "");
    let result = fixture.result(&["--max-used-space", "80GB"], 0);
    assert_eq!(
        statuses(&result),
        [("sha256:newer".into(), "selected".into())]
    );
    assert_eq!(result["summary"]["images"], 1);
    assert_eq!(result["within_budget"], false);
}

#[test]
fn unavailable_docker_and_invalid_budgets_remove_nothing() {
    let fixture = Fixture::new(&["100GB"], &crowded());
    fixture.write("unavailable", "");
    let result = fixture.result(&["--max-used-space", "80GB", "--apply"], 1);
    assert_eq!(result["status"], "failed");
    assert_eq!(result["error"], "Cannot connect to the Docker daemon");
    let out = fixture.run(&["--max-used-space", "80GB", "--apply"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out).contains("workctl: Cannot connect to the Docker daemon"));

    for invalid in [
        &["--apply"][..],
        &["--max-used-space", "80"],
        &["--max-used-space", "1.5GB"],
    ] {
        let out = fixture.run(invalid);
        assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    }
    assert!(fixture.removals().is_empty());
}

#[test]
fn large_selections_are_measured_between_bounded_batches() {
    let images: Vec<_> = (0..40)
        .map(|index| {
            let id: &'static str = Box::leak(format!("sha256:{index:02}").into_boxed_str());
            image(id, "2020-01-01T00:00:00Z", "1GB")
        })
        .collect();
    let fixture = Fixture::new(&["100GB", "100GB", "30GB"], &images);
    let result = fixture.result(&["--max-used-space", "50GB", "--apply"], 0);
    assert_eq!(result["summary"]["removed"], 40);
    let calls = fixture.calls();
    let measurements: Vec<_> = calls
        .iter()
        .enumerate()
        .filter(|(_, call)| call.starts_with("system df --format"))
        .map(|(index, _)| index)
        .collect();
    let removed_between = |from: usize, to: usize| {
        calls[from..to]
            .iter()
            .filter(|call| call.starts_with("image rm "))
            .count()
    };
    assert_eq!(measurements.len(), 3);
    assert_eq!(removed_between(measurements[0], measurements[1]), 32);
    assert_eq!(removed_between(measurements[1], measurements[2]), 8);
}

#[test]
fn unreadable_reports_remove_nothing_and_later_failures_keep_their_evidence() {
    let unreadable = [image("sha256:odd", "2020-01-01T00:00:00Z", "plenty")];
    let fixture = Fixture::new(&["100GB"], &unreadable);
    let result = fixture.result(&["--max-used-space", "80GB", "--apply"], 1);
    assert_eq!(result["error"], "Docker returned an unreadable image size");
    assert!(fixture.removals().is_empty());

    let fixture = Fixture::new(&["100GB", "fail"], &crowded());
    let result = fixture.result(&["--max-used-space", "80GB", "--apply"], 1);
    assert_eq!(result["status"], "failed");
    assert_eq!(result["error"], "daemon went away");
    assert_eq!(result["images"][0]["status"], "removed");
    assert_eq!(result["used_bytes_after"], Value::Null);
    let out =
        Fixture::new(&["100GB", "fail"], &crowded()).run(&["--max-used-space", "80GB", "--apply"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out).contains("workctl: daemon went away"),
        "{}",
        text(&out)
    );
    assert!(text(&out).contains("Removed        2"), "{}", text(&out));
}

#[test]
fn heartbeat_records_only_successful_applied_runs() {
    let fixture = Fixture::new(&["79.9GB"], &crowded());
    let stamp = fixture.state("state/docker-image-clean.succeeded");
    let args = ["--max-used-space", "80GB", "--apply", "--heartbeat"];

    fixture.write("unavailable", "");
    fixture.result(&args, 1);
    assert!(!stamp.exists(), "a failed run must not look healthy");

    // A run that starts removing and then meets a refusal is not healthy either.
    let partial = Fixture::new(&["100GB", "70GB"], &crowded());
    partial.write("refused", "sha256:newer\n");
    partial.write("listed", "sha256:newer\n");
    let result = partial.result(&args, 1);
    assert_eq!(result["summary"]["removed"], 1);
    assert_eq!(result["summary"]["failed"], 1);
    assert!(!partial.state("state/docker-image-clean.succeeded").exists());

    fs::remove_file(fixture.state("unavailable")).unwrap();
    let result = fixture.result(&args, 0);
    let recorded: u64 = fs::read_to_string(&stamp).unwrap().parse().unwrap();
    assert_eq!(result["started_unix_seconds"], recorded);

    fixture.result(&["--max-used-space", "80GB", "--apply"], 0);
    assert_eq!(fs::read_to_string(&stamp).unwrap(), recorded.to_string());

    let protected = [Image {
        containers: "1",
        ..image("sha256:protected", "2020-01-01T00:00:00Z", "95GB")
    }];
    let fixture = Fixture::new(&["95GB"], &protected);
    let result = fixture.result(&args, 0);
    assert_eq!(result["within_budget"], false);
    let stamp = fs::read_to_string(fixture.state("state/docker-image-clean.succeeded")).unwrap();
    assert_eq!(
        stamp,
        format!("{} over-budget", result["started_unix_seconds"])
    );
    let out = fixture.run(&["--max-used-space", "80GB", "--heartbeat"]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
}

#[test]
fn usage_reports_are_asked_again_when_docker_is_briefly_inconsistent() {
    // A concurrent build can make one report fail while snapshots change under it.
    let fixture = Fixture::new(&["fail", "fail", "79.9GB"], &crowded());
    let result = fixture.result(&["--max-used-space", "80GB", "--apply"], 0);
    assert_eq!(result["status"], "completed");
    assert_eq!(result["used_bytes"], 79_900_000_000u64);
    assert_eq!(
        fixture.calls(),
        ["system df --format {{json .}}"; 3],
        "three reports, no removal"
    );

    let fixture = Fixture::new(&["fail", "fail", "fail", "79.9GB"], &crowded());
    let result = fixture.result(&["--max-used-space", "80GB", "--apply"], 1);
    assert_eq!(result["error"], "daemon went away");
    assert_eq!(fixture.calls().len(), 3, "a persistent failure still fails");
}
