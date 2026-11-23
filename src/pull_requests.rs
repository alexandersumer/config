//! Repository-scoped Bitbucket Cloud approvals using the native bb OAuth cache.
mod author;
mod rate_limit;
mod team;

use crate::auth::{self, process};
use crate::{presentation, runtime};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const API: &str = "https://api.bitbucket.org/2.0";
const REQUEST_SECONDS: u64 = 30;
const WORKERS: usize = 4;
const IDENTITY_URL: &str = "https://api.bitbucket.org/2.0/user?fields=uuid";
const MAX_ATTEMPTS: u32 = 3;

#[derive(Debug, clap::Args)]
#[command(group(clap::ArgGroup::new("author_scope").required(true).args(["author_team", "author_email", "author_account_id", "all_authors"])))]
pub(crate) struct Options {
    /// Bitbucket Cloud repositories as WORKSPACE/REPOSITORY or https://bitbucket.org/WORKSPACE/REPOSITORY
    #[arg(required = true, num_args = 1.., value_parser = repository)]
    repositories: Vec<String>,
    /// Limit approvals to authors who are full members of this Atlassian team
    #[arg(long, value_name = "TEAM_ID", value_parser = team::team_id)]
    author_team: Option<String>,
    /// Limit approvals to one author by exact Atlassian account email (resolved through TWG)
    #[arg(long, value_name = "EMAIL", value_parser = author::email_address)]
    author_email: Option<String>,
    /// Limit approvals to one author by Atlassian account ID
    #[arg(long, value_name = "ACCOUNT_ID", value_parser = author::account_id)]
    author_account_id: Option<String>,
    /// Explicitly allow pull requests from any author
    #[arg(long)]
    all_authors: bool,
    /// Report eligible pull requests without sending approvals
    #[arg(long)]
    dry_run: bool,
    /// Maximum seconds to wait for another approval run (default: wait until it finishes)
    #[arg(long, value_name = "SECONDS")]
    lock_timeout: Option<u64>,
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
    #[serde(default)]
    participants: Vec<Participant>,
}

enum AuthorScope {
    Any,
    Team(BTreeSet<String>),
    Account(String),
}

impl AuthorScope {
    fn matches(&self, author: Option<&User>) -> bool {
        let id = author.and_then(|user| user.account_id.as_deref());
        match self {
            Self::Any => true,
            Self::Team(members) => id.is_some_and(|id| members.contains(id)),
            Self::Account(expected) => id == Some(expected),
        }
    }

    fn selector(&self) -> Option<String> {
        match self {
            Self::Any => None,
            Self::Team(members) => {
                let ids = members
                    .iter()
                    .map(|id| json!(id).to_string())
                    .collect::<Vec<_>>();
                Some(format!("author.account_id IN ({})", ids.join(",")))
            }
            Self::Account(id) => Some(format!("author.account_id={}", json!(id))),
        }
    }
}

#[derive(Deserialize)]
struct Participant {
    user: User,
    approved: bool,
}

fn approved_by(participants: &[Participant], user: &str) -> bool {
    participants
        .iter()
        .any(|p| p.user.uuid == user && p.approved)
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

struct CredentialState {
    token: Arc<str>,
    renewed: bool,
    failure: Option<String>,
}

struct Client {
    account: String,
    owner: Option<String>,
    credentials: Mutex<CredentialState>,
    session: process::Session,
    rate_limit: rate_limit::RateLimit,
    human: bool,
    progress: Mutex<Option<(presentation::Progress, usize)>>,
}

enum RequestError {
    Transport(i32),
    Http(u16),
    RateLimited(String),
    EligibilityExpired,
    Invalid(String),
}

impl RequestError {
    fn retryable(&self) -> bool {
        // A read may be repeated after a timeout, dropped/truncated transfer, or
        // transient service failure. Rate limiting uses the shared cooldown.
        matches!(
            self,
            Self::Transport(18 | 28 | 52 | 55 | 56 | 92) | Self::Http(408 | 500 | 502 | 503 | 504)
        )
    }

    fn message(self) -> String {
        match self {
            Self::Transport(code) => format!("Bitbucket HTTPS request failed (curl exit {code})"),
            Self::Http(code) => format!("Bitbucket API returned HTTP {code}"),
            Self::RateLimited(_) => "Bitbucket rate limited the request (HTTP 429)".into(),
            Self::EligibilityExpired => {
                "Approval eligibility must be rechecked after a cooldown or credential change"
                    .into()
            }
            Self::Invalid(reason) => reason,
        }
    }
}

impl Client {
    fn request<T: DeserializeOwned>(&self, url: &str) -> Result<T, String> {
        self.read_retry(|attempt| self.send("GET", url, attempt, None))
    }

    fn read_retry<T>(
        &self,
        mut issue: impl FnMut(u32) -> Result<T, RequestError>,
    ) -> Result<T, String> {
        for attempt in 0..MAX_ATTEMPTS {
            match issue(attempt) {
                Err(RequestError::RateLimited(_)) => {
                    if let Some(reason) = self.rate_limit.stop_reason() {
                        return Err(reason);
                    }
                }
                Err(RequestError::EligibilityExpired) if attempt + 1 < MAX_ATTEMPTS => {}
                Err(error)
                    if attempt + 1 < MAX_ATTEMPTS && error.retryable() && !runtime::cancelled() =>
                {
                    std::thread::sleep(Duration::from_millis(250 << attempt));
                }
                result => return result.map_err(RequestError::message),
            }
        }
        unreachable!("the final attempt returns its result")
    }

    fn token(&self) -> Result<Arc<str>, String> {
        let credentials = self.credentials.lock().expect("Bitbucket credentials");
        if let Some(reason) = &credentials.failure {
            return Err(reason.clone());
        }
        Ok(credentials.token.clone())
    }

    fn auth_failure(&self) -> Option<String> {
        self.credentials
            .lock()
            .expect("Bitbucket credentials")
            .failure
            .clone()
    }

    fn renew(&self, used: &Arc<str>, owner: &str) -> Result<(), String> {
        // Serialize recovery across workers. A 401 from an older in-flight
        // request must never trigger a second refresh of an already replaced token.
        let mut credentials = self.credentials.lock().expect("Bitbucket credentials");
        if let Some(reason) = &credentials.failure {
            return Err(reason.clone());
        }
        if !Arc::ptr_eq(used, &credentials.token) {
            return Ok(());
        }
        let renew = || -> Result<String, String> {
            if credentials.renewed {
                return Err("Bitbucket rejected the renewed OAuth credential".into());
            }
            let token = auth::bitbucket_token(&self.account)?;
            if token.as_str() == used.as_ref() {
                return Err("Bitbucket CLI did not provide a new token".into());
            }
            let identity: User = self.read_retry(|attempt| {
                self.rate_limit.wait().map_err(RequestError::Invalid)?;
                self.send_with_token("GET", IDENTITY_URL, attempt, &token)
            })?;
            if identity.uuid != owner {
                return Err(
                    "Renewed OAuth credential belongs to a different Bitbucket identity".into(),
                );
            }
            Ok(token)
        };
        match renew() {
            Ok(token) => {
                credentials.token = Arc::from(token);
                credentials.renewed = true;
                Ok(())
            }
            Err(reason) => {
                let reason = format!("{reason}. Recheck Bitbucket authentication before rerunning");
                credentials.failure = Some(reason.clone());
                Err(reason)
            }
        }
    }

    fn send<T: DeserializeOwned>(
        &self,
        method: &str,
        url: &str,
        attempt: u32,
        verified_token: Option<&Arc<str>>,
    ) -> Result<T, RequestError> {
        let token = self.token().map_err(RequestError::Invalid)?;
        let waited = self.rate_limit.wait().map_err(RequestError::Invalid)?;
        if verified_token.is_some_and(|verified| waited || !Arc::ptr_eq(verified, &token)) {
            return Err(RequestError::EligibilityExpired);
        }
        let result = self.send_with_token(method, url, attempt, &token);
        if matches!(result, Err(RequestError::Http(401))) {
            if let Some(owner) = &self.owner {
                self.renew(&token, owner).map_err(RequestError::Invalid)?;
                return Err(RequestError::EligibilityExpired);
            }
        }
        result
    }

    fn send_with_token<T: DeserializeOwned>(
        &self,
        method: &str,
        url: &str,
        attempt: u32,
        token: &str,
    ) -> Result<T, RequestError> {
        let result = self.request_once(method, url, token);
        if let Err(RequestError::RateLimited(header)) = &result {
            if let Some(delay) = self.rate_limit.limited(header, attempt) {
                if self.human {
                    let wait = if delay < Duration::from_secs(1) {
                        format!("{:.2}s", delay.as_secs_f64())
                    } else {
                        presentation::duration(delay)
                    };
                    let message =
                        format!("Bitbucket rate limit reached. Waiting {wait} before continuing.");
                    let progress = self.progress.lock().expect("approval progress");
                    let _ = match progress.as_ref() {
                        Some((progress, _)) => progress.message(&message),
                        None => writeln!(io::stderr().lock(), "{message}")
                            .map_err(|error| error.to_string()),
                    };
                }
            }
        }
        result
    }

    fn request_once<T: DeserializeOwned>(
        &self,
        method: &str,
        url: &str,
        token: &str,
    ) -> Result<T, RequestError> {
        let invalid = |reason: &str| RequestError::Invalid(reason.into());
        // Use a private socket for curl's config stdin. The bearer never enters
        // argv, the environment, a capture file, or a user-facing diagnostic.
        let (input, mut writer) =
            UnixStream::pair().map_err(|_| invalid("Cannot create HTTP input"))?;
        writeln!(writer, "header = \"Authorization: Bearer {token}\"")
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
            "\n%{http_code}\n%header{retry-after}",
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
        let mut parts = output.stdout.rsplitn(3, |byte| *byte == b'\n');
        let retry_after = parts
            .next()
            .and_then(|bytes| std::str::from_utf8(bytes).ok());
        let status = parts
            .next()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .and_then(|value| value.parse::<u16>().ok());
        if status == Some(429) {
            return Err(RequestError::RateLimited(
                retry_after.unwrap_or_default().into(),
            ));
        }
        if output.code != 0 {
            return Err(RequestError::Transport(output.code));
        }
        let code = status.ok_or_else(|| invalid("Bitbucket response has no HTTP status"))?;
        if code != 200 {
            return Err(RequestError::Http(code));
        }
        let body = parts
            .next()
            .ok_or_else(|| invalid("Bitbucket response has no body"))?;
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
            let page: Page<T> = self.request(&url)?;
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
        scope: &AuthorScope,
    ) -> Result<BTreeMap<u64, bool>, String> {
        let mut ids = BTreeMap::new();
        let reviewer = format!("reviewers.uuid=\"{me}\"");
        // Keep the URL bounded. Large teams still get the same local author filter.
        let selector = scope
            .selector()
            .map(|author| format!("{reviewer} AND {author}"))
            .filter(|selector| selector.len() <= 900)
            .unwrap_or(reviewer);
        let mut encoded = String::new();
        for byte in selector.bytes() {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                encoded.push(char::from(byte));
            } else {
                use std::fmt::Write;
                write!(&mut encoded, "%{byte:02X}").expect("encode Bitbucket query");
            }
        }
        let author_fields = if !matches!(scope, AuthorScope::Any) {
            ",values.author.uuid,values.author.account_id"
        } else {
            ""
        };
        let query = format!(
            "state=OPEN&sort=id&pagelen=50&fields=next,values.id,values.state,values.reviewers.uuid,values.participants.user.uuid,values.participants.approved{author_fields}&q={encoded}"
        );
        self.any_page(collection, &query, |listed: ListedPullRequest| {
            let pr = listed.candidate;
            if pr.id == 0 {
                return Err("Bitbucket returned an invalid pull request ID".into());
            }
            if pr.state == "OPEN"
                && pr.reviewers.iter().any(|user| user.uuid == me)
                && scope.matches(listed.author.as_ref())
            {
                let already_approved = approved_by(&listed.participants, me);
                ids.entry(pr.id)
                    .and_modify(|approved| *approved |= already_approved)
                    .or_insert(already_approved);
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
        scope: &AuthorScope,
    ) -> Result<&'static str, String> {
        let detail = format!("{endpoint}?fields=id,state,author.uuid,author.account_id,reviewers.uuid,participants.user.uuid,participants.approved");
        for attempt in 0..MAX_ATTEMPTS {
            let verified_token = self.token()?;
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
            let pr: PullRequest = self.request(&detail)?;
            if pr.candidate.id != id {
                return Err("Bitbucket returned a different pull request ID".into());
            }
            if pr.candidate.state != "OPEN"
                || !pr.candidate.reviewers.iter().any(|user| user.uuid == me)
                || pr.author.uuid == me
            {
                return Ok("no_longer_eligible");
            }
            if approved_by(&pr.participants, me) {
                return Ok("already_approved");
            }
            if !scope.matches(Some(&pr.author)) {
                return Ok(match scope {
                    AuthorScope::Team(_) => "author_outside_team",
                    _ => "author_mismatch",
                });
            }
            if dry_run {
                return Ok("would_approve");
            }

            let write = self.send::<Participant>(
                "POST",
                &format!("{endpoint}/approve"),
                attempt,
                Some(&verified_token),
            );
            if attempt + 1 < MAX_ATTEMPTS
                && matches!(
                    write,
                    Err(RequestError::RateLimited(_) | RequestError::EligibilityExpired)
                )
                && !self.rate_limit.paused()
            {
                continue;
            }
            // A dropped connection can leave a successful write. Read back the
            // outcome and never replay a POST that may have reached Bitbucket.
            let verified = self.request::<PullRequest>(&detail);
            if verified
                .as_ref()
                .is_ok_and(|pr| pr.candidate.id == id && approved_by(&pr.participants, me))
            {
                return Ok("approved");
            }
            let reason = write
                .err()
                .map(RequestError::message)
                .or_else(|| verified.err())
                .unwrap_or_else(|| "approval was absent from the read-back".into());
            return Err(format!("Approval could not be verified ({reason}). Inspect the pull request or rerun to check fresh state"));
        }
        unreachable!("the final attempt returns its result")
    }
}

fn supported_curl(output: &[u8]) -> bool {
    let text = String::from_utf8_lossy(output);
    let Some(version) = text.split_whitespace().nth(1) else {
        return false;
    };
    let mut numbers = version
        .split('.')
        .filter_map(|part| part.parse::<u32>().ok());
    numbers
        .next()
        .zip(numbers.next())
        .is_some_and(|pair| pair >= (7, 84))
}

fn approve(
    options: &Options,
    human: bool,
    repositories: &BTreeSet<&str>,
    results: &mut Vec<Value>,
    resolved_author: &mut Option<String>,
) -> Result<(), String> {
    let _signals =
        runtime::Signals::install().map_err(|_| "Cannot install cancellation handlers")?;
    // Live runs use the same lock as older releases to prevent duplicate approvals.
    let _lock = (!options.dry_run)
        .then(|| process::Lock::acquire_approval(options.lock_timeout))
        .transpose()?;
    // Check the transport before any credential refresh or browser interaction.
    let mut curl = Command::new("curl");
    curl.args(["--disable", "--version"]);
    let (code, version, _) = runtime::capture(curl, Duration::from_secs(5), true)?;
    if code != 0 || !supported_curl(&version) {
        return Err("curl 7.84 or later is required for Bitbucket rate-limit handling".into());
    }
    let author_scope = if let Some(id) = &options.author_team {
        let members = team::member_accounts(id, options.non_interactive, human)?;
        if members.is_empty() {
            return Ok(());
        }
        AuthorScope::Team(members)
    } else if let Some(email) = &options.author_email {
        let id = author::resolve_email(email, options.non_interactive, human)?;
        if human {
            writeln!(
                io::stderr().lock(),
                "Resolved author {email} to Atlassian account ID {id}"
            )
            .map_err(|_| "Cannot report the resolved author")?;
        }
        *resolved_author = Some(id.clone());
        AuthorScope::Account(id)
    } else if let Some(id) = &options.author_account_id {
        *resolved_author = Some(id.clone());
        AuthorScope::Account(id.clone())
    } else {
        AuthorScope::Any
    };

    let account = auth::bitbucket_account(options.non_interactive, human)?;
    let token = auth::bitbucket_token(&account)?;
    let mut client = Client {
        account,
        owner: None,
        credentials: Mutex::new(CredentialState {
            token: Arc::from(token),
            renewed: false,
            failure: None,
        }),
        session: process::Session::default(),
        rate_limit: rate_limit::RateLimit::default(),
        human,
        progress: Mutex::new(None),
    };
    let me: User = client.request(IDENTITY_URL)?;
    let uuid_pattern =
        regex::Regex::new(r"^\{[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}\}$")
            .expect("static Bitbucket UUID pattern");
    if !uuid_pattern.is_match(&me.uuid) {
        return Err("Bitbucket did not return a valid UUID for the authenticated user".into());
    }

    client.owner = Some(me.uuid.clone());

    // Discover at most four repositories at a time. Every repository must
    // finish discovery successfully before any worker can send an approval.
    if human {
        writeln!(io::stderr().lock(), "Discovering pull requests…")
            .map_err(|_| "Cannot report pull request discovery")?;
        *client.progress.lock().expect("approval progress") =
            Some((presentation::Progress::new(repositories.len()), 0));
    }
    let collections: Vec<_> = repositories
        .iter()
        .map(|&repo| (repo, format!("{API}/repositories/{repo}/pullrequests")))
        .collect();
    let mut discovered = Vec::new();
    for group in collections.chunks(WORKERS) {
        std::thread::scope(|scope| -> Result<(), String> {
            let workers: Vec<_> = group
                .iter()
                .map(|(repo, collection)| {
                    let client = &client;
                    let reviewer = &me.uuid;
                    let author_scope = &author_scope;
                    scope.spawn(move || {
                        let ids = client
                            .assigned_ids(collection, reviewer, author_scope)
                            .map_err(|reason| format!("{repo}: {reason}"))?;
                        Ok::<_, String>(
                            ids.into_iter()
                                .map(|(id, approved)| {
                                    (*repo, format!("{collection}/{id}"), id, approved)
                                })
                                .collect::<Vec<_>>(),
                        )
                    })
                })
                .collect();
            let active = workers.len();
            for (index, worker) in workers.into_iter().enumerate() {
                discovered.extend(worker.join().expect("repository discovery worker")?);
                if let Some((display, completed)) =
                    client.progress.lock().expect("approval progress").as_mut()
                {
                    *completed += 1;
                    display.update(*completed, active - index - 1)?;
                }
            }
            Ok(())
        })?;
    }
    client.progress.lock().expect("approval progress").take();
    if human && !discovered.is_empty() {
        let suffix = if discovered.len() == 1 { "" } else { "s" };
        writeln!(
            io::stderr().lock(),
            "Checking {} pull request{suffix}…",
            discovered.len()
        )
        .map_err(|_| "Cannot report pull request progress")?;
        *client.progress.lock().expect("approval progress") =
            Some((presentation::Progress::new(discovered.len()), 0));
    }
    let next = AtomicUsize::new(0);
    let progress_failed = AtomicBool::new(false);
    let mut indexed = Vec::new();
    std::thread::scope(|scope| {
        let mut workers = Vec::new();
        for _ in 0..WORKERS.min(discovered.len()) {
            workers.push(scope.spawn(|| {
                let mut outcomes = Vec::new();
                while !runtime::cancelled() && !client.rate_limit.paused() && client.auth_failure().is_none() && !progress_failed.load(Ordering::Relaxed) {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some((repo, endpoint, id, already_approved)) = discovered.get(index) else { break };
                    let mut row = json!({"repository":repo,"id":id,"url":format!("https://bitbucket.org/{repo}/pull-requests/{id}")});
                    if *already_approved {
                        row["status"] = json!("already_approved");
                    } else {
                        match client.approval_status(endpoint, *id, &me.uuid, options.dry_run, &author_scope) {
                            Ok(status) => row["status"] = json!(status),
                            Err(reason) => {
                                row["status"] = json!("failed");
                                row["reason"] = json!(reason);
                            }
                        }
                    }
                    outcomes.push((index, row));
                    if human {
                        let mut progress = client.progress.lock().expect("approval progress");
                        if let Some((display, completed)) = progress.as_mut() {
                            *completed += 1;
                            let active = next.load(Ordering::Relaxed).min(discovered.len()).saturating_sub(*completed);
                            if display.update(*completed, active).is_err() {
                                progress_failed.store(true, Ordering::Relaxed);
                            }
                        }
                    }
                }
                outcomes
            }));
        }
        for worker in workers {
            indexed.extend(worker.join().expect("approval worker"));
        }
    });
    client.progress.lock().expect("approval progress").take();
    indexed.sort_unstable_by_key(|(index, _)| *index);
    results.extend(indexed.into_iter().map(|(_, result)| result));
    let remaining = discovered.len() - results.len();
    if let Some(reason) = client
        .rate_limit
        .stop_reason()
        .or_else(|| client.auth_failure())
    {
        return Err(format!(
            "{reason}. {remaining} pull requests were left pending."
        ));
    }
    if runtime::cancelled() {
        return Err(format!(
            "Interrupted with {remaining} pull requests remaining"
        ));
    }
    if progress_failed.load(Ordering::Relaxed) {
        return Err(format!(
            "Cannot report pull request progress. {remaining} pull requests were left pending."
        ));
    }
    Ok(())
}

fn report(
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    results: &[Value],
    dry_run: bool,
    status: &str,
    error: Option<&str>,
    elapsed: Duration,
) -> io::Result<()> {
    if let Some(reason) = error {
        writeln!(stderr, "workctl: {}", presentation::safe_text(reason))?;
    }
    let (mut changed, mut prior, mut skipped, mut failed) = (0, 0, 0, 0);
    for row in results {
        let repository = row["repository"].as_str().unwrap_or_default();
        let id = &row["id"];
        match row["status"].as_str() {
            Some("would_approve") => {
                changed += 1;
                writeln!(stdout, "{repository} #{id}  would approve")?;
            }
            Some("approved") => changed += 1,
            Some("previously_approved" | "already_approved") => prior += 1,
            Some("failed") => {
                failed += 1;
                let reason = row["reason"].as_str().unwrap_or("Approval failed");
                writeln!(
                    stderr,
                    "workctl: {repository} #{id}: {}",
                    presentation::safe_text(reason)
                )?;
            }
            _ => skipped += 1,
        }
    }
    let heading = match status {
        "interrupted" => "Approval interrupted",
        "failed" => "Approval incomplete",
        _ if dry_run => "Approval preview",
        _ => "Approval complete",
    };
    let changed_label = if dry_run { "eligible" } else { "approved" };
    write!(stdout, "{heading} · {changed} {changed_label}")?;
    if prior > 0 {
        write!(
            stdout,
            " · {prior} prior approval{}",
            if prior == 1 { "" } else { "s" }
        )?;
    }
    if skipped > 0 {
        write!(stdout, " · {skipped} no longer eligible")?;
    }
    if failed > 0 {
        write!(stdout, " · {failed} failed")?;
    }
    writeln!(stdout, " · {}", presentation::duration(elapsed))?;
    stderr.flush()?;
    stdout.flush()
}

pub(crate) fn run(options: Options, json_output: bool) -> u8 {
    let started = Instant::now();
    let repositories: BTreeSet<_> = options.repositories.iter().map(String::as_str).collect();
    let mut results = Vec::new();
    let mut resolved_author = None;
    let outcome = approve(
        &options,
        !json_output,
        &repositories,
        &mut results,
        &mut resolved_author,
    );
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
        if let Some(id) = resolved_author {
            report["author_account_id"] = json!(id);
        }
        presentation::json(&report)
    } else {
        u8::from(
            report(
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
                &results,
                options.dry_run,
                status,
                error.as_deref(),
                started.elapsed(),
            )
            .is_err(),
        )
    };
    if interrupted {
        130
    } else {
        output.max(u8::from(failed))
    }
}
