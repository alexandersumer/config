//! Repository-scoped Bitbucket Cloud approvals using the native bb OAuth cache.
mod team;

use crate::auth::{self, process};
use crate::{presentation, runtime};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::io::{self, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::time::Duration;

const API: &str = "https://api.bitbucket.org/2.0";
const REQUEST_SECONDS: u64 = 30;

#[derive(Debug, clap::Args)]
pub(crate) struct Options {
    /// Bitbucket Cloud repositories as WORKSPACE/REPOSITORY or https://bitbucket.org/WORKSPACE/REPOSITORY
    #[arg(required = true, num_args = 1.., value_parser = repository)]
    repositories: Vec<String>,
    /// Limit approvals to authors who are full members of this Atlassian team
    #[arg(long, value_name = "TEAM_ID", value_parser = team::team_id)]
    author_team: Option<String>,
    /// Report eligible pull requests without sending approvals
    #[arg(long)]
    dry_run: bool,
    /// Allow silent OAuth refresh without browser login
    #[arg(long)]
    non_interactive: bool,
}

fn repository(value: &str) -> Result<String, String> {
    let value = value
        .strip_prefix("https://bitbucket.org/")
        .unwrap_or(value);
    let value = value.strip_suffix('/').unwrap_or(value);
    let slug = |part: &str| {
        part.as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    };
    if value
        .split_once('/')
        .is_some_and(|(workspace, repo)| slug(workspace) && slug(repo))
    {
        Ok(value.to_ascii_lowercase())
    } else {
        Err("expected WORKSPACE/REPOSITORY or a Bitbucket Cloud repository URL".into())
    }
}

#[derive(Deserialize)]
struct User {
    uuid: String,
    account_id: Option<String>,
}

#[derive(Deserialize)]
struct Page<T> {
    values: Vec<T>,
    next: Option<String>,
}

#[derive(Deserialize)]
struct Candidate {
    id: u64,
    state: String,
    reviewers: Vec<User>,
}

#[derive(Deserialize)]
struct ListedPullRequest {
    #[serde(flatten)]
    candidate: Candidate,
    author: Option<User>,
}

fn author_matches(author: Option<&User>, team: Option<&BTreeSet<String>>) -> bool {
    team.is_none_or(|members| {
        author
            .and_then(|user| user.account_id.as_ref())
            .is_some_and(|id| members.contains(id))
    })
}

#[derive(Deserialize)]
struct Participant {
    user: User,
    approved: bool,
}

#[derive(Deserialize)]
struct PullRequest {
    #[serde(flatten)]
    candidate: Candidate,
    author: User,
    participants: Vec<Participant>,
}

#[derive(Deserialize)]
struct Approval {
    user: User,
}

#[derive(Deserialize)]
struct Activity {
    approval: Option<Approval>,
}

struct Client {
    token: String,
    session: process::Session,
}

enum RequestError {
    Transport(i32),
    Http(u16),
    Invalid(String),
}

impl RequestError {
    fn retryable(&self) -> bool {
        // A read may be repeated after a timeout, dropped/truncated transfer, or
        // transient service failure. HTTP 429 needs provider-directed backoff.
        matches!(
            self,
            Self::Transport(18 | 28 | 52 | 55 | 56 | 92) | Self::Http(408 | 500 | 502 | 503 | 504)
        )
    }

    fn message(self) -> String {
        match self {
            Self::Transport(code) => format!("Bitbucket HTTPS request failed (curl exit {code})"),
            Self::Http(code) => format!("Bitbucket API returned HTTP {code}"),
            Self::Invalid(reason) => reason,
        }
    }
}

impl Client {
    fn request<T: DeserializeOwned>(&self, method: &str, url: &str) -> Result<T, String> {
        let mut retries = 0;
        loop {
            match self.request_once(method, url) {
                Err(error)
                    if method == "GET"
                        && retries < 2
                        && error.retryable()
                        && !runtime::cancelled() =>
                {
                    std::thread::sleep(Duration::from_millis(250 << retries));
                    retries += 1;
                }
                result => return result.map_err(RequestError::message),
            }
        }
    }

    fn request_once<T: DeserializeOwned>(
        &self,
        method: &str,
        url: &str,
    ) -> Result<T, RequestError> {
        let invalid = |reason: &str| RequestError::Invalid(reason.into());
        // Use a private socket for curl's config stdin. The bearer never enters
        // argv, the environment, a capture file, or a user-facing diagnostic.
        let (input, mut writer) =
            UnixStream::pair().map_err(|_| invalid("Cannot create HTTP input"))?;
        writeln!(writer, "header = \"Authorization: Bearer {}\"", self.token)
            .map_err(|_| invalid("Cannot prepare HTTP authentication"))?;
        drop(writer);
        let mut command = Command::new("curl");
        command.args([
            "--disable",
            "--config",
            "-",
            "--silent",
            "--globoff",
            "--proto",
            "=https",
            "--connect-timeout",
            "10",
            "--max-time",
            "28",
            "--max-filesize",
            "250000",
            "--header",
            "Accept: application/json",
            "--request",
            method,
            "--write-out",
            "\n%{http_code}",
            "--",
            url,
        ]);
        let output = process::capture_with_stdin(
            &self.session,
            "Bitbucket API",
            command,
            REQUEST_SECONDS,
            None,
            Stdio::from(OwnedFd::from(input)),
        )
        .map_err(RequestError::Invalid)?;
        if output.code != 0 {
            return Err(RequestError::Transport(output.code));
        }
        let split = output
            .stdout
            .iter()
            .rposition(|b| *b == b'\n')
            .ok_or_else(|| invalid("Bitbucket response has no HTTP status"))?;
        let (body, status) = (&output.stdout[..split], &output.stdout[split + 1..]);
        if status != b"200" {
            let code = std::str::from_utf8(status)
                .ok()
                .and_then(|s| s.parse::<u16>().ok())
                .ok_or_else(|| invalid("Invalid Bitbucket HTTP status"))?;
            return Err(RequestError::Http(code));
        }
        serde_json::from_slice(body)
            .map_err(|_| invalid("Bitbucket response did not match the expected JSON schema"))
    }

    fn any_page<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query: &str,
        mut visit: impl FnMut(T) -> Result<bool, String>,
    ) -> Result<bool, String> {
        let mut url = format!("{endpoint}?{query}");
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(url.clone()) {
                return Err("Bitbucket pagination repeated a page".into());
            }
            let page: Page<T> = self.request("GET", &url)?;
            for item in page.values {
                if visit(item)? {
                    return Ok(true);
                }
            }
            let Some(next) = page.next else {
                return Ok(false);
            };
            // Pagination links may contain opaque cursors. Only the query of
            // the exact collection can change, never the host or path.
            if !next.strip_prefix(endpoint).is_some_and(|s| {
                s.starts_with('?') && s.bytes().all(|b| b.is_ascii_graphic() && b != b'#')
            }) {
                return Err("Bitbucket pagination escaped the requested collection".into());
            }
            url = next;
        }
    }

    fn assigned_ids(
        &self,
        collection: &str,
        me: &str,
        team: Option<&BTreeSet<String>>,
    ) -> Result<BTreeSet<u64>, String> {
        let mut ids = BTreeSet::new();
        let uuid = &me[1..me.len() - 1]; // validated when resolving the current user
        let author_fields = if team.is_some() {
            ",values.author.uuid,values.author.account_id"
        } else {
            ""
        };
        let query = format!(
            "state=OPEN&sort=id&pagelen=50&fields=next,values.id,values.state,values.reviewers.uuid{author_fields}&q=reviewers.uuid%3D%22%7B{uuid}%7D%22"
        );
        self.any_page(collection, &query, |listed: ListedPullRequest| {
            let pr = listed.candidate;
            if pr.id == 0 {
                return Err("Bitbucket returned an invalid pull request ID".into());
            }
            if pr.state == "OPEN"
                && pr.reviewers.iter().any(|user| user.uuid == me)
                && author_matches(listed.author.as_ref(), team)
            {
                ids.insert(pr.id);
            }
            Ok(false)
        })?;
        Ok(ids)
    }

    fn approval_status(
        &self,
        endpoint: &str,
        id: u64,
        me: &str,
        dry_run: bool,
        team: Option<&BTreeSet<String>>,
    ) -> Result<&'static str, String> {
        let approved_before = self.any_page(
            &format!("{endpoint}/activity"),
            "pagelen=50&fields=next,values.approval.user.uuid",
            |event: Activity| {
                Ok(event
                    .approval
                    .is_some_and(|approval| approval.user.uuid == me))
            },
        )?;
        if approved_before {
            return Ok("previously_approved");
        }
        let detail = format!("{endpoint}?fields=id,state,author.uuid,author.account_id,reviewers.uuid,participants.user.uuid,participants.approved");
        let pr: PullRequest = self.request("GET", &detail)?;
        if pr.candidate.id != id {
            return Err("Bitbucket returned a different pull request ID".into());
        }
        if pr.candidate.state != "OPEN"
            || !pr.candidate.reviewers.iter().any(|user| user.uuid == me)
            || pr.author.uuid == me
        {
            return Ok("no_longer_eligible");
        }
        if pr
            .participants
            .iter()
            .any(|p| p.user.uuid == me && p.approved)
        {
            return Ok("already_approved");
        }
        if !author_matches(Some(&pr.author), team) {
            return Ok("author_outside_team");
        }
        if dry_run {
            return Ok("would_approve");
        }

        // A dropped connection can leave a successful write. Read back before
        // deciding the outcome. This invocation never replays an approval POST.
        let write = self.request::<Participant>("POST", &format!("{endpoint}/approve"));
        let verified = self.request::<PullRequest>("GET", &detail);
        if verified.as_ref().is_ok_and(|pr| {
            pr.candidate.id == id
                && pr
                    .participants
                    .iter()
                    .any(|p| p.user.uuid == me && p.approved)
        }) {
            return Ok("approved");
        }
        let reason = write
            .err()
            .or_else(|| verified.err())
            .unwrap_or_else(|| "approval was absent from the read-back".into());
        Err(format!("Approval could not be verified ({reason}). Inspect the pull request or rerun to check fresh state"))
    }
}

fn approve(
    options: &Options,
    human: bool,
    repositories: &BTreeSet<&str>,
    results: &mut Vec<Value>,
) -> Result<(), String> {
    let _signals =
        runtime::Signals::install().map_err(|_| "Cannot install cancellation handlers")?;
    // Retain the same lock across versions so old and new invocations coordinate.
    let _lock = process::Lock::acquire_for("pr-approve", 30)?;
    // Check the transport before any credential refresh or browser interaction.
    let mut curl = Command::new("curl");
    curl.args(["--disable", "--version"]);
    let (code, _, _) = runtime::capture(curl, Duration::from_secs(5), true)?;
    if code != 0 {
        return Err("A working curl is required for the Bitbucket API".into());
    }
    let roster = options
        .author_team
        .as_deref()
        .map(|id| team::member_accounts(id, options.non_interactive, human))
        .transpose()?;
    if roster.as_ref().is_some_and(BTreeSet::is_empty) {
        return Ok(());
    }

    let client = Client {
        token: auth::bitbucket_token(options.non_interactive, human)?,
        session: process::Session::default(),
    };
    let me: User = client.request("GET", &format!("{API}/user?fields=uuid"))?;
    let uuid_pattern =
        regex::Regex::new(r"^\{[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}\}$")
            .expect("static Bitbucket UUID pattern");
    if !uuid_pattern.is_match(&me.uuid) {
        return Err("Bitbucket did not return a valid UUID for the authenticated user".into());
    }

    // Complete discovery for every explicit repository before issuing any POST.
    let mut discovered = Vec::new();
    for repo in repositories {
        if human {
            writeln!(
                io::stderr().lock(),
                "Checking open pull requests in {repo}…"
            )
            .map_err(|_| "Cannot report pull request discovery")?;
        }
        let collection = format!("{API}/repositories/{repo}/pullrequests");
        let ids = client
            .assigned_ids(&collection, &me.uuid, roster.as_ref())
            .map_err(|reason| format!("{repo}: {reason}"))?;
        discovered.push((repo, collection, ids));
    }
    for (repo, collection, ids) in discovered {
        for id in ids {
            if runtime::cancelled() {
                return Err("Interrupted with pull requests remaining".into());
            }
            if human {
                writeln!(io::stderr().lock(), "Checking {repo} #{id}…")
                    .map_err(|_| "Cannot report pull request progress")?;
            }
            let endpoint = format!("{collection}/{id}");
            let mut row = json!({"repository":repo,"id":id,"url":format!("https://bitbucket.org/{repo}/pull-requests/{id}")});
            match client.approval_status(&endpoint, id, &me.uuid, options.dry_run, roster.as_ref())
            {
                Ok(status) => row["status"] = json!(status),
                Err(reason) => {
                    row["status"] = json!("failed");
                    row["reason"] = json!(reason);
                }
            }
            results.push(row);
        }
    }
    Ok(())
}

pub(crate) fn run(options: Options, json_output: bool) -> u8 {
    let repositories: BTreeSet<_> = options.repositories.iter().map(String::as_str).collect();
    let mut results = Vec::new();
    let outcome = approve(&options, !json_output, &repositories, &mut results);
    let interrupted = runtime::interrupted();
    let failed = outcome.is_err() || results.iter().any(|row| row["status"] == "failed");
    let status = if interrupted {
        "interrupted"
    } else if failed {
        "failed"
    } else {
        "completed"
    };
    let error = outcome.err();
    let output = if json_output {
        // Keep the operation identifier stable for existing schema-version 1 readers.
        let mut report = json!({"schema_version":1,"operation":"pr.approve","status":status,"dry_run":options.dry_run,"repositories":repositories,"results":results,"error":error});
        if let Some(team_id) = &options.author_team {
            report["author_team"] = json!(team_id);
        }
        presentation::json(&report)
    } else {
        let render = || -> io::Result<()> {
            if let Some(reason) = error {
                writeln!(
                    io::stderr().lock(),
                    "workctl: {}",
                    presentation::safe_text(&reason)
                )?;
            }
            let mut stdout = io::stdout().lock();
            for row in &results {
                write!(
                    stdout,
                    "{} #{}  {}",
                    row["repository"].as_str().unwrap_or_default(),
                    row["id"],
                    row["status"].as_str().unwrap_or_default().replace('_', " ")
                )?;
                if let Some(reason) = row["reason"].as_str() {
                    write!(stdout, " ({})", presentation::safe_text(reason))?;
                }
                writeln!(stdout)?;
            }
            if results.is_empty() && !failed && !interrupted {
                if let Some(team_id) = &options.author_team {
                    writeln!(
                        stdout,
                        "No open pull requests assigned to you from Atlassian team {team_id}."
                    )?;
                } else {
                    writeln!(stdout, "No open pull requests assigned to you.")?;
                }
            }
            Ok(())
        };
        u8::from(render().is_err())
    };
    if interrupted {
        130
    } else {
        output.max(u8::from(failed))
    }
}
