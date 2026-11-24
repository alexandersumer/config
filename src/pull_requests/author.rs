//! Resolve an exact Atlassian account email to the ID used by Bitbucket.
use crate::auth::{self, process};
use serde::Deserialize;
use serde_json::Value;
use std::process::Command;

pub(super) fn account_id(value: &str) -> Result<String, String> {
    if !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b":_-".contains(&b))
    {
        Ok(value.to_owned())
    } else {
        Err("expected an Atlassian account ID; use --author-email for an email address".into())
    }
}

pub(super) fn email_address(value: &str) -> Result<String, String> {
    if value.len() <= 64
        && value.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_+-".contains(&b))
        })
    {
        return Ok(format!("{value}@atlassian.com"));
    }
    if value.len() <= 254
        && value.bytes().all(|b| b.is_ascii_graphic())
        && value.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && domain.contains('.')
                && domain.split('.').all(|label| {
                    !label.is_empty()
                        && label
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                })
        })
    {
        Ok(value.to_owned())
    } else {
        Err("expected a full email address or the part before @atlassian.com".into())
    }
}

#[derive(Deserialize)]
struct DirectoryResponse {
    #[serde(rename = "apiVersion")]
    version: String,
    command: String,
    data: DirectoryAccount,
    meta: DirectoryMeta,
    error: Option<Value>,
    #[serde(default)]
    errors: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryAccount {
    account_id: String,
    account_status: String,
    email: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryMeta {
    result_count: usize,
    #[serde(default)]
    partial: bool,
    #[serde(default)]
    truncated: bool,
}

pub(super) fn resolve_email(
    email: &str,
    non_interactive: bool,
    human: bool,
) -> Result<String, String> {
    auth::prepare_profile("twg", non_interactive, human)?;
    let mut command = Command::new("twg");
    command
        .args([
            "user",
            "get",
            "--mode",
            "user",
            "-o",
            "json",
            "--output-summary",
            "none",
            "--",
            email,
        ])
        .env("TWG_AGENT_DEFAULTS", "0");
    let output = process::capture(
        &process::Session::default(),
        "Atlassian user directory",
        command,
        30,
        None,
    )?;
    let failure = "TWG could not verify one active Atlassian account with that exact email. Check the email or use --author-account-id with a verified ID";
    if output.code != 0 {
        return Err(failure.into());
    }
    let response: DirectoryResponse =
        serde_json::from_slice(&output.stdout).map_err(|_| failure)?;
    if response.version != "v2"
        || response.command != "user.get"
        || response.error.is_some()
        || !response.errors.is_empty()
        || response.meta.result_count != 1
        || response.meta.partial
        || response.meta.truncated
        || response.data.account_status != "active"
        || !response.data.email.eq_ignore_ascii_case(email)
    {
        return Err(failure.into());
    }
    account_id(&response.data.account_id).map_err(|_| failure.into())
}
