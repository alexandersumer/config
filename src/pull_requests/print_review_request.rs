use super::{check_curl, repository, Client, API};
use crate::{presentation, runtime};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::{self, Write as _};

#[derive(Debug, clap::Args)]
pub(crate) struct Options {
    /// Bitbucket Cloud pull request URLs or WORKSPACE/REPOSITORY#ID references
    #[arg(required = true, num_args = 1.., value_name = "PULL_REQUEST", value_parser = pull_request)]
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
}

fn build_message(options: Options, human: bool) -> Result<String, String> {
    check_curl()?;
    let client = Client::connect(options.non_interactive, human)?;
    let mut seen = BTreeSet::new();
    let mut message = String::from("Could someone please review these?");
    for reference in &options.pull_requests {
        if !seen.insert(reference) {
            continue;
        }
        let Reference { repository, id } = reference;
        let details: Details = client
            .request(&format!(
                "{API}/repositories/{repository}/pullrequests/{id}?fields=id,state,title"
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
        let title = details
            .title
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if title.is_empty() {
            return Err(format!(
                "{repository}#{id}: Bitbucket returned an empty pull request title"
            ));
        }
        write!(message, "\n#{id}: {}", presentation::safe_text(&title)).expect("write to String");
    }
    Ok(message)
}

pub(crate) fn run(options: Options, json_output: bool) -> u8 {
    let outcome = (|| {
        let _signals =
            runtime::Signals::install().map_err(|_| "Cannot install cancellation handlers")?;
        let outcome = build_message(options, !json_output);
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
