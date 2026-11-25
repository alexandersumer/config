//! Local Docker image retention. Docker never removes unused images itself, and
//! `docker image prune --filter until=` compares build time, so this holds the
//! image store to a size budget by when each image reached this machine.
use crate::{presentation, runtime};
use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write as _};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, clap::Args)]
pub(crate) struct CleanArgs {
    /// Space the local image store may keep (for example 80GB)
    #[arg(long, value_name = "SIZE", value_parser = budget)]
    max_used_space: u64,
    /// Select arrival timestamps strictly before one invocation cutoff (for example 2h)
    #[arg(long, value_parser = crate::workspace::duration)]
    older_than: Option<Duration>,
    /// Remove the selected images; Docker still refuses any image a container uses
    #[arg(long)]
    apply: bool,
    /// Record a successful run for the overdue checks in `workctl doctor --managed` and the shell
    #[arg(long, requires = "apply")]
    heartbeat: bool,
    /// Deadline per Docker command (1-86400 seconds); removal never retries
    #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u32).range(1..=86_400), value_name = "SECONDS")]
    timeout: u32,
}

/// A whole decimal size with its unit, as Docker writes budgets (`80GB`).
fn budget(value: &str) -> Result<u64, String> {
    let digits = value.bytes().take_while(u8::is_ascii_digit).count();
    let multiplier: u64 = match value[digits..].to_ascii_lowercase().as_str() {
        "b" => 1,
        "kb" => 1_000,
        "mb" => 1_000_000,
        "gb" => 1_000_000_000,
        "tb" => 1_000_000_000_000,
        _ => return Err("expected a size such as 80GB (B, kB, MB, GB, TB)".into()),
    };
    value[..digits]
        .parse::<u64>()
        .ok()
        .and_then(|number| number.checked_mul(multiplier))
        .ok_or_else(|| "size must be a whole number followed by its unit".into())
}

/// Docker reports sizes as rounded `%.4g` decimals: `396.7GB`, `1e+03MB`, and,
/// for layers it attributes elsewhere, negative values such as `-2.222e+07B`.
fn reported_size(text: &str) -> Option<u64> {
    let number = text.trim().strip_suffix('B')?;
    let (number, scale) = match number.as_bytes().last()? {
        b'k' => (&number[..number.len() - 1], 1e3),
        b'M' => (&number[..number.len() - 1], 1e6),
        b'G' => (&number[..number.len() - 1], 1e9),
        b'T' => (&number[..number.len() - 1], 1e12),
        b'P' => (&number[..number.len() - 1], 1e15),
        _ => (number, 1.0),
    };
    let value = number
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())?;
    Some((value * scale).max(0.0) as u64)
}

/// Unix seconds of an RFC 3339 timestamp. Docker's zero time means unknown.
fn unix_seconds(text: &str) -> Option<i64> {
    let number = |range: std::ops::Range<usize>| {
        text.get(range)
            .filter(|part| part.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|part| part.parse::<i64>().ok())
    };
    let separator = |index: usize, expected: &[u8]| {
        text.as_bytes()
            .get(index)
            .is_some_and(|byte| expected.contains(byte))
    };
    if !(separator(4, b"-")
        && separator(7, b"-")
        && separator(10, b"Tt ")
        && separator(13, b":")
        && separator(16, b":"))
    {
        return None;
    }
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    // Every byte before the fraction is ASCII, so this boundary is valid.
    let mut zone = &text[19..];
    if let Some(fraction) = zone.strip_prefix('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        zone = &fraction[digits..];
    }
    let offset = if matches!(zone, "Z" | "z") {
        0
    } else {
        let sign = match zone.as_bytes().first()? {
            b'+' => 1,
            b'-' => -1,
            _ => return None,
        };
        if zone.len() != 6 || zone.as_bytes()[3] != b':' {
            return None;
        }
        let field = |range: std::ops::Range<usize>| {
            zone.get(range)
                .filter(|part| part.bytes().all(|byte| byte.is_ascii_digit()))
                .and_then(|part| part.parse::<i64>().ok())
        };
        sign * (field(1..3)? * 3600 + field(4..6)? * 60)
    };
    // Days from the civil date, counting years from March so leap days fall last.
    let (year, month) = if month <= 2 {
        (year - 1, month + 9)
    } else {
        (year, month - 3)
    };
    let year_of_era = year.rem_euclid(400);
    let days = year.div_euclid(400) * 146_097 + year_of_era * 365 + year_of_era / 4
        - year_of_era / 100
        + (153 * month + 2) / 5
        + day
        - 1
        - 719_468;
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second - offset;
    (seconds > 0).then_some(seconds)
}

/// When an unattended run last succeeded, in Unix seconds.
pub(crate) fn last_success() -> Option<u64> {
    let stamp = crate::history::state_file(HEARTBEAT).ok()?;
    std::fs::read_to_string(stamp).ok()?.trim().parse().ok()
}

const HEARTBEAT: &str = "docker-image-clean.succeeded";

fn record_success(started: u64) -> Result<(), String> {
    let stamp = crate::history::state_file(HEARTBEAT)?;
    let mut staged = tempfile::NamedTempFile::new_in(stamp.parent().expect("state directory"))
        .map_err(|error| format!("heartbeat: {error}"))?;
    staged
        .write_all(started.to_string().as_bytes())
        .and_then(|_| staged.as_file().sync_all())
        .map_err(|error| format!("heartbeat: {error}"))?;
    staged
        .persist(stamp)
        .map_err(|error| format!("heartbeat: {error}"))?;
    Ok(())
}

fn human(bytes: u64) -> String {
    for (unit, scale) in [("TB", 1e12), ("GB", 1e9), ("MB", 1e6), ("kB", 1e3)] {
        if bytes as f64 >= scale {
            return format!("{:.1} {unit}", bytes as f64 / scale);
        }
    }
    format!("{bytes} B")
}

/// Distinct repositories among `name:tag` and `name@digest` references.
fn repositories(references: &[String]) -> usize {
    references
        .iter()
        .map(|reference| {
            let name = reference.split('@').next().unwrap_or(reference);
            match name.rfind(':') {
                Some(colon) if !name[colon..].contains('/') => &name[..colon],
                _ => name,
            }
        })
        .collect::<BTreeSet<_>>()
        .len()
}

#[derive(Deserialize)]
struct Usage {
    #[serde(rename = "Type")]
    kind: String,
    #[serde(rename = "Size")]
    size: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Report {
    images: Vec<Row>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Row {
    #[serde(rename = "ID")]
    id: String,
    unique_size: String,
    containers: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Details {
    id: String,
    repo_tags: Option<Vec<String>>,
    repo_digests: Option<Vec<String>>,
    #[serde(default)]
    metadata: Metadata,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Metadata {
    #[serde(default)]
    last_tag_time: String,
}

struct Image {
    id: String,
    references: Vec<String>,
    arrived: String,
    arrived_unix: Option<i64>,
    unique: u64,
    in_use: bool,
}

struct Docker {
    timeout: Duration,
}

impl Docker {
    fn run(&self, args: &[&str]) -> Result<(i32, Vec<u8>, Vec<u8>), String> {
        let mut command = std::process::Command::new("docker");
        command.args(args);
        runtime::capture(command, self.timeout, true).map_err(|error| format!("docker: {error}"))
    }

    fn output(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        match self.run(args)? {
            (0, stdout, _) => Ok(stdout),
            (_, _, stderr) => Err(failure(&stderr)),
        }
    }

    /// Docker's usage report can fail while a concurrent build or removal changes
    /// the snapshots it is measuring. Reports only read, so ask again before
    /// treating one such moment as a failed run.
    fn report(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        let mut attempt = 1;
        loop {
            match self.output(args) {
                Err(_) if attempt < 3 && !runtime::cancelled() => {
                    attempt += 1;
                    std::thread::sleep(Duration::from_secs(1));
                }
                report => return report,
            }
        }
    }

    fn used(&self) -> Result<u64, String> {
        let report = self.report(&["system", "df", "--format", "{{json .}}"])?;
        for line in report.split(|byte| *byte == b'\n') {
            if line.is_empty() {
                continue;
            }
            let usage: Usage = serde_json::from_slice(line)
                .map_err(|_| "Docker returned an unreadable disk usage report")?;
            if usage.kind == "Images" {
                return reported_size(&usage.size)
                    .ok_or_else(|| "Docker returned an unreadable image store size".into());
            }
        }
        Err("Docker's disk usage report has no image total".into())
    }

    fn images(&self) -> Result<Vec<Image>, String> {
        let report = self.report(&["system", "df", "--verbose", "--format", "{{json .}}"])?;
        let report: Report = serde_json::from_slice(&report)
            .map_err(|_| "Docker returned an unreadable image usage report")?;
        let mut details = BTreeMap::new();
        for rows in report.images.chunks(100) {
            let mut args = vec!["image", "inspect"];
            args.extend(rows.iter().map(|row| row.id.as_str()));
            // Inspect exits unsuccessfully when an image vanished after the usage
            // report, while still describing the others.
            let (_, stdout, stderr) = self.run(&args)?;
            let found: Vec<Details> =
                serde_json::from_slice(&stdout).map_err(|_| failure(&stderr))?;
            details.extend(found.into_iter().map(|image| (image.id.clone(), image)));
        }
        let mut images = Vec::new();
        for row in report.images {
            let Some(image) = details.remove(&row.id) else {
                continue;
            };
            let mut references = image.repo_tags.unwrap_or_default();
            references.extend(image.repo_digests.unwrap_or_default());
            images.push(Image {
                arrived_unix: unix_seconds(&image.metadata.last_tag_time),
                arrived: image.metadata.last_tag_time,
                // A size read as zero would select every image to cover the excess.
                unique: reported_size(&row.unique_size)
                    .ok_or("Docker returned an unreadable image size")?,
                // An unreadable count protects the image.
                in_use: row
                    .containers
                    .parse::<u64>()
                    .map_or(true, |count| count > 0),
                id: row.id,
                references,
            });
        }
        Ok(images)
    }

    /// Never forced: Docker itself refuses while a container uses the image.
    fn remove(&self, image: &Image) -> Removal {
        // Docker removes by ID only when one repository names the image.
        let targets: Vec<&str> = if repositories(&image.references) > 1 {
            image.references.iter().map(String::as_str).collect()
        } else {
            vec![image.id.as_str()]
        };
        let mut refusal = None;
        for target in targets {
            match self.run(&["image", "rm", target]) {
                Ok((0, _, _)) => {}
                Ok((_, _, stderr)) => refusal = Some(failure(&stderr)),
                Err(error) => return Removal::Failed(error),
            }
        }
        let Some(refusal) = refusal else {
            return Removal::Removed;
        };
        // Removing the last tag also drops the digest references that followed it.
        match self.output(&["image", "ls", "--all", "--quiet", "--no-trunc"]) {
            Ok(remaining)
                if !String::from_utf8_lossy(&remaining)
                    .lines()
                    .any(|id| id == image.id) =>
            {
                return Removal::Removed
            }
            Ok(_) => {}
            Err(error) => return Removal::Failed(error),
        }
        let ancestor = format!("ancestor={}", image.id);
        match self.output(&["container", "ls", "--all", "--quiet", "--filter", &ancestor]) {
            Ok(containers) if !containers.trim_ascii().is_empty() => Removal::InUse,
            _ => Removal::Failed(refusal),
        }
    }
}

enum Removal {
    Removed,
    InUse,
    Failed(String),
}

fn failure(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map_or_else(|| "docker exited unsuccessfully".into(), str::to_owned)
}

/// Images removed between measurements. Unique sizes leave out layers that
/// candidates share with each other, so this bounds how far a batch can overshoot.
const BATCH: usize = 32;

/// How many of the oldest arrivals cover the excess by their unique sizes.
fn covering(candidates: &[Image], excess: u64) -> usize {
    let mut covered = 0u64;
    candidates
        .iter()
        .take_while(|image| {
            let needed = covered < excess;
            covered = covered.saturating_add(image.unique);
            needed
        })
        .count()
}

#[derive(Default)]
struct Outcome {
    used: u64,
    used_after: Option<u64>,
    images: usize,
    in_use: usize,
    recent: usize,
    unknown_arrival: usize,
    records: Vec<(Image, &'static str, Option<String>)>,
    /// A failure after removal began; the records above still happened.
    error: Option<String>,
}

fn clean(args: &CleanArgs) -> Result<Outcome, String> {
    let docker = Docker {
        timeout: Duration::from_secs(args.timeout.into()),
    };
    let mut outcome = Outcome {
        used: docker.used()?,
        ..Outcome::default()
    };
    if outcome.used <= args.max_used_space {
        return Ok(outcome);
    }
    let cutoff = args.older_than.map(|age| {
        SystemTime::now()
            .checked_sub(age)
            .and_then(|cutoff| cutoff.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |cutoff| cutoff.as_secs() as i64)
    });
    let images = docker.images()?;
    outcome.images = images.len();
    let mut candidates = Vec::new();
    for image in images {
        match image.arrived_unix {
            _ if image.in_use => outcome.in_use += 1,
            None => outcome.unknown_arrival += 1,
            Some(arrived) if cutoff.is_some_and(|cutoff| arrived >= cutoff) => outcome.recent += 1,
            Some(_) => candidates.push(image),
        }
    }
    candidates.sort_by(|a, b| (a.arrived_unix, &a.id).cmp(&(b.arrived_unix, &b.id)));
    let mut candidates = candidates.into_iter();
    let mut used = outcome.used;
    // Unique sizes are estimates, so measure again after each batch until the
    // store fits or nothing removable is left.
    while used > args.max_used_space && !runtime::cancelled() {
        let count = covering(candidates.as_slice(), used - args.max_used_space);
        if !args.apply {
            outcome.records.extend(
                candidates
                    .by_ref()
                    .take(count)
                    .map(|image| (image, "selected", None)),
            );
            break;
        }
        let batch: Vec<_> = candidates.by_ref().take(count.min(BATCH)).collect();
        if batch.is_empty() {
            break;
        }
        writeln!(
            io::stderr(),
            "Removing {} image{}…",
            batch.len(),
            if batch.len() == 1 { "" } else { "s" }
        )
        .map_err(|e| e.to_string())?;
        for image in batch {
            if runtime::cancelled() {
                break;
            }
            let (status, error) = match docker.remove(&image) {
                Removal::Removed => ("removed", None),
                Removal::InUse => ("in_use", None),
                Removal::Failed(error) => ("failed", Some(error)),
            };
            outcome.records.push((image, status, error));
        }
        match docker.used() {
            Ok(measured) => used = measured,
            Err(error) => {
                outcome.error = Some(error);
                break;
            }
        }
        outcome.used_after = Some(used);
    }
    Ok(outcome)
}

pub(crate) fn run(args: CleanArgs, json_output: bool) -> u8 {
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let outcome = (|| {
        let _signals =
            runtime::Signals::install().map_err(|_| "Cannot install cancellation handlers")?;
        clean(&args)
    })();
    let count = |outcome: &Outcome, wanted: &str| {
        outcome
            .records
            .iter()
            .filter(|(_, status, _)| *status == wanted)
            .count()
    };
    let failed = outcome.as_ref().map_or(true, |outcome| {
        outcome.error.is_some() || count(outcome, "failed") != 0
    });
    // A heartbeat that cannot be written must not look like a healthy run.
    let outcome = match outcome {
        Ok(outcome) if args.heartbeat && !failed && !runtime::interrupted() => {
            record_success(started).map(|()| outcome)
        }
        outcome => outcome,
    };
    let failed = failed || outcome.is_err();
    let (status, code) = if runtime::interrupted() {
        ("interrupted", 130)
    } else if failed {
        ("failed", 1)
    } else {
        ("completed", 0)
    };
    let estimate = |outcome: &Outcome| {
        outcome
            .records
            .iter()
            .fold(0u64, |sum, (image, _, _)| sum.saturating_add(image.unique))
    };
    // A preview projects from estimates; an applied run reports what Docker measured.
    let remaining = |outcome: &Outcome| {
        outcome
            .used_after
            .unwrap_or_else(|| outcome.used.saturating_sub(estimate(outcome)))
    };
    let output = if json_output {
        let result = match &outcome {
            Ok(outcome) => json!({
                "error": outcome.error,
                "used_bytes": outcome.used,
                "used_bytes_after": outcome.used_after,
                "within_budget": remaining(outcome) <= args.max_used_space,
                "summary": {
                    "images": outcome.images,
                    "in_use": outcome.in_use,
                    "recent": outcome.recent,
                    "unknown_arrival": outcome.unknown_arrival,
                    "selected": outcome.records.len(),
                    "removed": count(outcome, "removed"),
                    "skipped": count(outcome, "in_use"),
                    "failed": count(outcome, "failed"),
                    "estimated_bytes": estimate(outcome),
                },
                "images": outcome.records.iter().map(|(image, status, error)| json!({
                    "id": image.id,
                    "references": image.references,
                    "arrived": image.arrived,
                    "unique_bytes": image.unique,
                    "status": status,
                    "error": error,
                })).collect::<Vec<_>>(),
            }),
            Err(error) => json!({ "error": error }),
        };
        let mut document = json!({
            "schema_version": 1,
            "operation": "docker.image.clean",
            "status": status,
            "started_unix_seconds": started,
            "apply": args.apply,
            "max_used_space_bytes": args.max_used_space,
            "older_than_seconds": args.older_than.map(|age| age.as_secs()),
        });
        document
            .as_object_mut()
            .expect("result object")
            .extend(result.as_object().expect("result object").clone());
        presentation::json(&document)
    } else {
        match &outcome {
            Ok(outcome) => {
                let mut text = format!(
                    "Docker image cleanup{}\n\nBudget         {}\nUsed           {}\n",
                    if args.apply { "" } else { " preview" },
                    human(args.max_used_space),
                    human(outcome.used)
                );
                if outcome.used > args.max_used_space {
                    text.push_str(&format!(
                        "Images         {} ({} used by containers, {} inside the age limit, {} of unknown arrival)\n",
                        outcome.images, outcome.in_use, outcome.recent, outcome.unknown_arrival
                    ));
                    if args.apply {
                        text.push_str(&format!(
                            "Removed        {}\nIn use         {}\nFailed         {}\nUsed after     {}\n",
                            count(outcome, "removed"),
                            count(outcome, "in_use"),
                            count(outcome, "failed"),
                            human(remaining(outcome))
                        ));
                    } else {
                        text.push_str(&format!(
                            "Selected       {} (about {})\n",
                            outcome.records.len(),
                            human(estimate(outcome))
                        ));
                    }
                    if remaining(outcome) > args.max_used_space {
                        text.push_str(&format!(
                            "Over budget    about {}; every other image is in use, inside the age limit, or of unknown arrival\n",
                            human(remaining(outcome) - args.max_used_space)
                        ));
                    }
                }
                for (image, status, error) in &outcome.records {
                    let name = image
                        .references
                        .first()
                        .map_or("<untagged>", String::as_str);
                    text.push_str(&format!(
                        "\n  {status:<8} {} {:>9}  {}",
                        image.arrived.get(..10).unwrap_or("unknown   "),
                        human(image.unique),
                        presentation::safe_text(name)
                    ));
                    if let Some(error) = error {
                        text.push_str(&format!("\n    {}", presentation::safe_text(error)));
                    }
                }
                if !outcome.records.is_empty() {
                    text.push('\n');
                    if !args.apply {
                        text.push_str(&format!(
                            "\nNext: workctl docker image clean --max-used-space {}B{} --apply\n",
                            args.max_used_space,
                            args.older_than.map_or_else(String::new, |age| format!(
                                " --older-than {}s",
                                age.as_secs()
                            ))
                        ));
                    }
                }
                if let Some(error) = &outcome.error {
                    let _ = writeln!(
                        io::stderr().lock(),
                        "workctl: {}",
                        presentation::safe_text(error)
                    );
                }
                let verdict = match code {
                    130 => "Interrupted",
                    0 => "Completed",
                    _ => "Failed",
                };
                let mut stdout = io::stdout().lock();
                u8::from(
                    writeln!(
                        stdout,
                        "{text}\n{}",
                        presentation::status(verdict, code != 0, false)
                    )
                    .and_then(|_| stdout.flush())
                    .is_err(),
                )
            }
            Err(error) => {
                let _ = writeln!(
                    io::stderr().lock(),
                    "workctl: {}",
                    presentation::safe_text(error)
                );
                1
            }
        }
    };
    code.max(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budgets_require_a_whole_number_and_decimal_unit() {
        assert_eq!(budget("80GB"), Ok(80_000_000_000));
        assert_eq!(budget("500mb"), Ok(500_000_000));
        assert_eq!(budget("0B"), Ok(0));
        for invalid in [
            "",
            "80",
            "GB",
            "1.5GB",
            "-1GB",
            "80GiB",
            "80 GB",
            "99999999999TB",
        ] {
            assert!(budget(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn reported_sizes_cover_docker_rounding_exponents_and_negative_attribution() {
        assert_eq!(reported_size("396.7GB"), Some(396_700_000_000));
        assert_eq!(reported_size("53.25kB"), Some(53_250));
        assert_eq!(reported_size("0B"), Some(0));
        assert_eq!(reported_size("1e+03MB"), Some(1_000_000_000));
        assert_eq!(reported_size("-2.222e+07B"), Some(0));
        for invalid in ["", "GB", "12", "1.2XB", "NaNB", "infB"] {
            assert_eq!(reported_size(invalid), None, "{invalid}");
        }
    }

    #[test]
    fn arrival_times_parse_as_rfc_3339_and_zero_time_is_unknown() {
        assert_eq!(unix_seconds("2000-01-01T00:00:00Z"), Some(946_684_800));
        assert_eq!(unix_seconds("2000-01-01T11:00:00+11:00"), Some(946_684_800));
        assert_eq!(
            unix_seconds("1999-12-31T19:00:00.5-05:00"),
            Some(946_684_800)
        );
        assert_eq!(
            unix_seconds("2026-11-24T03:54:13.926334835Z"),
            Some(1_795_492_453)
        );
        assert_eq!(unix_seconds("2024-02-29T12:00:00Z"), Some(1_709_208_000));
        for unknown in [
            "",
            "0001-01-01T00:00:00Z",
            "2026-11-24",
            "2026-13-01T00:00:00Z",
            "2026-11-24T03:54:13",
            "2026-11-24T03:54:13.Z",
            "2026-11-24T03:54:13+1100",
            "2026-11-24T03:54:13é",
        ] {
            assert_eq!(unix_seconds(unknown), None, "{unknown}");
        }
    }

    #[test]
    fn references_count_repositories_not_tags_or_registry_ports() {
        let references = |names: &[&str]| {
            names
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(repositories(&references(&[])), 0);
        assert_eq!(
            repositories(&references(&["app:1", "app:2", "app@sha256:abc"])),
            1
        );
        assert_eq!(
            repositories(&references(&["localhost:5000/app:1", "localhost:5000/app"])),
            1
        );
        assert_eq!(
            repositories(&references(&["app:1", "registry.example/team/app:1"])),
            2
        );
    }

    #[test]
    fn selection_takes_the_oldest_arrivals_until_estimates_cover_the_excess() {
        let image = |unique| Image {
            id: String::new(),
            references: vec![],
            arrived: String::new(),
            arrived_unix: Some(1),
            unique,
            in_use: false,
        };
        let candidates = [image(40), image(0), image(70), image(10)];
        assert_eq!(covering(&candidates, 0), 0);
        assert_eq!(covering(&candidates, 40), 1);
        assert_eq!(covering(&candidates, 41), 3);
        assert_eq!(covering(&candidates, 1_000), 4);
    }

    #[test]
    fn sizes_render_in_decimal_units() {
        assert_eq!(human(396_700_000_000), "396.7 GB");
        assert_eq!(human(1_500), "1.5 kB");
        assert_eq!(human(999), "999 B");
    }
}
