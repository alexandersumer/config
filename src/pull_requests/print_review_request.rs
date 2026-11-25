use super::{check_curl, pull_request_url, repository, valid_slug, Client, User, API};
use crate::{presentation, runtime};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeSet;
use std::io::{self, Write as _};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const WORKSPACE_WORKERS: usize = 12;

#[derive(Debug, clap::Args)]
pub(crate) struct Options {
    /// Bitbucket pull request URLs or WORKSPACE/REPOSITORY#ID references (default: all open, unqueued PRs you authored)
    #[arg(value_name = "PULL_REQUEST", value_parser = pull_request)]
    pull_requests: Vec<Reference>,
    /// Allow silent OAuth refresh without browser login
    #[arg(long)]
    non_interactive: bool,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Reference {
    repository: String,
    id: u64,
}

fn pull_request(value: &str) -> Result<Reference, String> {
    let invalid =
        || "expected a Bitbucket Cloud pull request URL or WORKSPACE/REPOSITORY#ID".to_owned();
    let (repo, id) = if let Some(url) = value.strip_prefix("https://bitbucket.org/") {
        let path = url.split(['?', '#']).next().expect("URL path");
        path.strip_suffix('/')
            .unwrap_or(path)
            .split_once("/pull-requests/")
    } else {
        value.split_once('#')
    }
    .ok_or_else(invalid)?;
    if repo.split('/').count() != 2 {
        return Err(invalid());
    }
    let repository = repository(repo).map_err(|_| invalid())?;
    let id = id
        .parse::<u64>()
        .ok()
        .filter(|&number| number != 0 && id.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(invalid)?;
    Ok(Reference { repository, id })
}

#[derive(Deserialize)]
struct Details {
    id: u64,
    state: String,
    title: String,
    queued: bool,
}

#[derive(Deserialize)]
struct Membership {
    workspace: Workspace,
}

#[derive(Deserialize)]
struct Workspace {
    slug: String,
}

#[derive(Deserialize)]
struct AuthoredPullRequest {
    #[serde(flatten)]
    details: Details,
    author: User,
    destination: Destination,
}

#[derive(Deserialize)]
struct Destination {
    repository: Repository,
}

#[derive(Deserialize)]
struct Repository {
    full_name: String,
}

fn append_pull_request(
    message: &mut String,
    reference: &Reference,
    title: &str,
) -> Result<(), String> {
    let Reference { repository, id } = reference;
    let mut title = title.trim();
    if title.is_empty() {
        return Err(format!(
            "{repository}#{id}: Bitbucket returned an empty pull request title"
        ));
    }

    while let Some((badge, rest)) = title.strip_prefix('[').and_then(|s| s.split_once(']')) {
        let metadata_badges = [
            "no-risk",
            "low-risk",
            "medium-risk",
            "high-risk",
            "critical-risk",
            "noissue",
        ];
        if !metadata_badges
            .iter()
            .any(|metadata| badge.trim().eq_ignore_ascii_case(metadata))
        {
            break;
        }
        title = rest.trim_start();
    }

    while let Some((rest, badge)) = title.strip_suffix(']').and_then(|s| s.rsplit_once('[')) {
        if !badge.trim().eq_ignore_ascii_case("noissue") {
            break;
        }
        title = rest.trim_end();
    }

    message.push('\n');
    if !title.is_empty() {
        let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
        message.push_str(&presentation::safe_text(&title));
        message.push(' ');
    }
    message.push_str(&pull_request_url(repository, *id));
    Ok(())
}

fn append_selected(
    client: &Client,
    references: &[Reference],
    message: &mut String,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for reference in references {
        if !seen.insert(reference) {
            continue;
        }
        let Reference { repository, id } = reference;
        let details: Details = client
            .request(&format!(
                "{API}/repositories/{repository}/pullrequests/{id}?fields=id,state,title,queued"
            ))
            .map_err(|error| format!("{repository}#{id}: {error}"))?;
        if details.id != *id {
            return Err(format!(
                "{repository}#{id}: Bitbucket returned a different pull request ID"
            ));
        }
        if details.state != "OPEN" {
            return Err(format!("{repository}#{id}: pull request is no longer open"));
        }
        if !details.queued {
            append_pull_request(message, reference, &details.title)?;
        }
    }
    Ok(())
}

fn append_authored(client: &Client, message: &mut String) -> Result<(), String> {
    let mut workspaces = BTreeSet::new();
    client
        .any_page(
            &format!("{API}/user/workspaces"),
            "pagelen=50&fields=next,values.workspace.slug",
            |membership: Membership| {
                let slug = membership.workspace.slug;
                if !valid_slug(&slug) {
                    return Err("Bitbucket returned an invalid workspace slug".into());
                }
                workspaces.insert(slug.to_ascii_lowercase());
                Ok(false)
            },
        )
        .map_err(|error| format!("Cannot list Bitbucket workspaces: {error}"))?;

    let workspaces: Vec<_> = workspaces.into_iter().collect();
    let author = client
        .owner
        .as_deref()
        .expect("verified Bitbucket identity");
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let workspaces = workspaces.as_slice();
    let mut results = std::thread::scope(|scope| {
        let mut workers = Vec::new();
        for _ in 0..WORKSPACE_WORKERS.min(workspaces.len()) {
            let worker = std::thread::Builder::new()
                .spawn_scoped(scope, || {
                    let mut completed = Vec::new();
                    while !stop.load(Ordering::Relaxed) && !runtime::cancelled() {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(workspace) = workspaces.get(index) else {
                            break;
                        };
                        let result = workspace_titles(client, workspace, author);
                        if result.is_err() {
                            stop.store(true, Ordering::Relaxed);
                        }
                        completed.push((index, result));
                    }
                    completed
                })
                .map_err(|error| {
                    stop.store(true, Ordering::Relaxed);
                    format!("Cannot start Bitbucket discovery worker: {error}")
                })?;
            workers.push(worker);
        }

        let mut results = Vec::with_capacity(workspaces.len());
        for worker in workers {
            results.extend(
                worker
                    .join()
                    .map_err(|_| "Bitbucket discovery worker failed")?,
            );
        }
        Ok::<_, String>(results)
    })?;

    let checked = results.len();
    results.sort_unstable_by_key(|(index, _)| *index);
    for (_, result) in results {
        message.push_str(&result?);
    }
    if checked != workspaces.len() {
        return Err("Bitbucket discovery stopped before checking all workspaces".into());
    }
    Ok(())
}

fn workspace_titles(client: &Client, workspace: &str, author: &str) -> Result<String, String> {
    let uuid = author.trim_matches(['{', '}']);
    let mut message = String::new();
    let mut seen = BTreeSet::new();
    client
        .any_page(
            &format!("{API}/workspaces/{workspace}/pullrequests/%7B{uuid}%7D"),
            "state=OPEN&pagelen=50&fields=next,values.id,values.state,values.title,values.queued,values.author.uuid,values.destination.repository.full_name",
            |pr: AuthoredPullRequest| {
                if pr.details.state != "OPEN" || pr.details.queued || pr.author.uuid != author {
                    return Ok(false);
                }
                let full_name = pr.destination.repository.full_name;
                let same_workspace = full_name
                    .split_once('/')
                    .is_some_and(|(slug, _)| slug.eq_ignore_ascii_case(workspace));
                if !same_workspace || pr.details.id == 0 {
                    return Err("Bitbucket returned an invalid pull request reference".into());
                }
                let reference = Reference {
                    repository: repository(&full_name)?,
                    id: pr.details.id,
                };
                if !seen.contains(&reference) {
                    append_pull_request(&mut message, &reference, &pr.details.title)?;
                    seen.insert(reference);
                }
                Ok(false)
            },
        )
        .map_err(|error| format!("{workspace}: {error}"))?;
    Ok(message)
}

fn build_message(options: Options) -> Result<String, String> {
    check_curl()?;
    let client = Client::connect(options.non_interactive, false)?;
    let mut message = String::new();
    if options.pull_requests.is_empty() {
        append_authored(&client, &mut message)?;
    } else {
        append_selected(&client, &options.pull_requests, &mut message)?;
    }
    Ok(if message.is_empty() {
        "No open pull requests available for review.".into()
    } else {
        format!("Could someone please review these?{message}")
    })
}

pub(crate) fn run(options: Options, json_output: bool) -> u8 {
    let outcome = (|| {
        let _signals =
            runtime::Signals::install().map_err(|_| "Cannot install cancellation handlers")?;
        let outcome = build_message(options);
        if runtime::interrupted() {
            Err("Interrupted while preparing the review request".into())
        } else {
            outcome
        }
    })();
    let (status, code) = if runtime::interrupted() {
        ("interrupted", 130)
    } else if outcome.is_err() {
        ("failed", 1)
    } else {
        ("completed", 0)
    };
    let output = if json_output {
        let (message, error) = match outcome {
            Ok(message) => (Some(message), None),
            Err(error) => (None, Some(error)),
        };
        presentation::json(&json!({
            "schema_version": 1,
            "operation": "pr.print-review-request",
            "status": status,
            "message": message,
            "error": error,
        }))
    } else {
        match outcome {
            Ok(message) => {
                let mut stdout = io::stdout().lock();
                u8::from(
                    writeln!(stdout, "{message}")
                        .and_then(|_| stdout.flush())
                        .is_err(),
                )
            }
            Err(error) => {
                let _ = writeln!(io::stderr().lock(), "{}", presentation::safe_text(&error));
                1
            }
        }
    };
    code.max(output)
}
