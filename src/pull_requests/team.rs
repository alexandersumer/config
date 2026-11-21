//! Resolve an explicit Atlassian team through TWG's native directory.
use crate::auth::{self, process};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::process::Command;

const PAGE_SIZE: &str = "100";
const MAX_PAGES: usize = 100;

pub(super) fn team_id(value: &str) -> Result<String, String> {
    let pattern = regex::Regex::new(r"^[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}$")
        .expect("static team UUID pattern");
    if pattern.is_match(value) {
        Ok(value.to_ascii_lowercase())
    } else {
        Err("expected an Atlassian team ID in UUID form".into())
    }
}

#[derive(Deserialize)]
struct DirectoryResponse {
    #[serde(rename = "apiVersion")]
    version: String,
    command: String,
    data: DirectoryTeam,
    error: Option<serde_json::Value>,
    #[serde(default)]
    errors: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct DirectoryTeam {
    id: String,
    state: String,
    members: DirectoryMembers,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryMembers {
    nodes: Vec<DirectoryMembership>,
    page_info: DirectoryPage,
}

#[derive(Deserialize)]
struct DirectoryMembership {
    state: String,
    member: Option<DirectoryAccount>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryAccount {
    account_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryPage {
    has_next_page: bool,
    end_cursor: Option<String>,
}

pub(super) fn member_accounts(
    id: &str,
    non_interactive: bool,
    human: bool,
) -> Result<BTreeSet<String>, String> {
    auth::prepare_profile("twg", non_interactive, human)?;
    let expected = format!("ari:cloud:identity::team/{id}");
    let session = process::Session::default();
    let mut accounts = BTreeSet::new();
    let mut seen_cursors = BTreeSet::new();
    let mut after: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let mut command = Command::new("twg");
        command
            .args([
                "teams",
                "members",
                "list",
                "--team-id",
                id,
                "--first",
                PAGE_SIZE,
            ])
            .args(["--mode", "user", "-o", "json", "--output-summary", "none"])
            .env("TWG_AGENT_DEFAULTS", "0");
        if let Some(cursor) = &after {
            command.arg(format!("--after={cursor}"));
        }
        let output = process::capture(&session, "Atlassian team directory", command, 30, None)?;
        if output.code != 0 {
            return Err(format!(
                "TWG could not read Atlassian team {id} (exit {})",
                output.code
            ));
        }
        let response: DirectoryResponse = serde_json::from_slice(&output.stdout)
            .map_err(|_| "TWG returned an invalid team membership response")?;
        if response.version != "v2"
            || response.command != "teams.members.list"
            || response.error.is_some()
            || !response.errors.is_empty()
            || response.data.id != expected
            || response.data.state != "ACTIVE"
        {
            return Err(format!(
                "TWG did not verify the requested active Atlassian team {id}"
            ));
        }
        for membership in response.data.members.nodes {
            if membership.state == "FULL_MEMBER" {
                let account = membership
                    .member
                    .filter(|member| {
                        !member.account_id.is_empty()
                            && member.account_id.trim() == member.account_id
                    })
                    .ok_or("TWG did not identify a full team member")?;
                accounts.insert(account.account_id);
            }
        }
        let page = response.data.members.page_info;
        if !page.has_next_page {
            return Ok(accounts);
        }
        let cursor = page
            .end_cursor
            .filter(|cursor| !cursor.trim().is_empty())
            .ok_or("TWG omitted the next team membership cursor")?;
        if !seen_cursors.insert(cursor.clone()) {
            return Err("TWG repeated a team membership cursor".into());
        }
        after = Some(cursor);
    }
    Err("TWG team membership exceeded the pagination limit".into())
}
