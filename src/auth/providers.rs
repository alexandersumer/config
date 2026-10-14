//! Narrow, explicit adapters for supported CLIs. Provider output is never rendered.
use super::config::{Options, Target};
use super::process::{self, Output};
use base64::Engine;
use serde_json::{json, Value};
use std::io::Write;
use std::process::Command;

#[derive(Debug)]
struct Evidence {
    remaining: Option<u64>,
    account: Option<String>,
}
enum Probe {
    Valid(Evidence),
    Missing,
    MissingForAccount(String),
    Failed(&'static str),
}
fn command(program: &str, args: &[&str]) -> Command {
    let mut c = Command::new(program);
    c.args(args);
    c
}
fn call(c: Command, options: &Options, login: bool) -> Result<Output, String> {
    process::capture(
        c,
        if login {
            options.login_timeout
        } else {
            options.timeout
        },
        login,
    )
}
fn auth_error(output: &Output) -> bool {
    let message = format!(
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .to_ascii_lowercase();
    [
        "auth_invalid",
        "invalid token",
        "token is invalid",
        "token is expired",
        "token has expired",
        "no existing token",
        "no cached token",
        "no accounts",
        "not logged in",
        "not authenticated",
        "authentication failed",
        "no active account",
        "no credentials",
        "no authentication",
    ]
    .iter()
    .any(|s| message.contains(s))
}
fn bb_status(target: &Target, options: &Options) -> Result<Output, String> {
    let mut c = command("bb", &["auth", "status"]);
    if let Target::Bitbucket {
        account: Some(account),
    } = target
    {
        c.args(["--account", account]);
    }
    call(c, options, false)
}
fn bitbucket(output: &Output, requested: Option<&str>) -> Probe {
    if output.code != 0 {
        return if auth_error(output) {
            Probe::Missing
        } else {
            Probe::Failed("Bitbucket status failed; check account access and CLI version")
        };
    }
    let text = String::from_utf8_lossy(&output.stdout);
    // Unknown/changed status formats fail closed instead of treating exit 0 as auth.
    let active =
        regex::Regex::new(r"(?m)^\s*✓?\s*([A-Za-z0-9._@-]+)\s+\(active\)").expect("static regex");
    let line = if let Some(account) = requested {
        text.lines().find(|line| {
            line.trim_start_matches(['✓', ' '])
                .starts_with(&format!("{account} "))
                || line
                    .trim_start_matches(['✓', ' '])
                    .starts_with(&format!("{account}\t"))
        })
    } else {
        text.lines().find(|line| active.is_match(line))
    };
    let Some(line) = line else {
        return Probe::Failed("Cannot identify the selected Bitbucket account from status output");
    };
    let account = requested
        .map(str::to_owned)
        .or_else(|| active.captures(line).map(|c| c[1].to_owned()));
    if line.to_ascii_lowercase().contains("expired") {
        return Probe::Valid(Evidence {
            remaining: Some(0),
            account,
        });
    }
    let Some((_, duration)) = line.split_once("expires in ") else {
        return Probe::Failed("Bitbucket token expiry unavailable; cannot verify readiness");
    };
    let Some(remaining) = duration_seconds(duration) else {
        return Probe::Failed("Unsupported Bitbucket token expiry format");
    };
    Probe::Valid(Evidence {
        remaining: Some(remaining),
        account,
    })
}
fn duration_seconds(text: &str) -> Option<u64> {
    let units = regex::Regex::new(r"(\d+)\s*(hours?|hrs?|h|minutes?|mins?|m|seconds?|secs?|s)\b")
        .expect("static regex");
    let mut total = 0u64;
    let mut end = 0;
    for c in units.captures_iter(text) {
        let span = c.get(0)?;
        if !text[end..span.start()].trim().is_empty() {
            return None;
        }
        let scale = if c[2].starts_with('h') {
            3600
        } else if c[2].starts_with('m') {
            60
        } else {
            1
        };
        total = total.checked_add(c[1].parse::<u64>().ok()?.checked_mul(scale)?)?;
        end = span.end();
    }
    (end > 0 && text[end..].trim().is_empty()).then_some(total)
}
fn twg_command(args: &[&str]) -> Command {
    let mut c = command("twg", args);
    c.args(["-o", "json", "--output-summary", "none"])
        .env("TWG_AGENT_DEFAULTS", "0");
    c
}
fn twig_probe(options: &Options) -> Result<Probe, String> {
    let output = call(twg_command(&["doctor"]), options, false)?;
    let Ok(value) = serde_json::from_slice::<Value>(&output.stdout) else {
        return Ok(Probe::Failed(
            "TWG did not return its documented JSON status",
        ));
    };
    if value
        .pointer("/data/connectivity/ok")
        .and_then(Value::as_bool)
        == Some(true)
    {
        // Doctor can exit 1 solely because its separate Bitbucket token is invalid.
        return Ok(Probe::Valid(Evidence {
            remaining: None,
            account: None,
        }));
    }
    if auth_error(&output)
        || value
            .pointer("/data/auth/resolved/tokenPresent")
            .and_then(Value::as_bool)
            == Some(false)
    {
        return Ok(Probe::Missing);
    }
    Ok(Probe::Failed(
        "TWG access check failed; inspect connectivity or permissions with twg doctor",
    ))
}
fn slauth_command(target: &Target, cached: bool) -> Command {
    let Target::Slauth {
        audience,
        environment,
        groups,
        mfa,
    } = target
    else {
        unreachable!()
    };
    let mut c = command("atlas", &["slauth", "token"]);
    c.args(["--aud", audience, "--env", environment, "--output", "jwt"]);
    if !groups.is_empty() {
        c.args(["--groups", &groups.join(",")]);
    }
    if *mfa {
        c.arg("--mfa");
    }
    c.arg(if cached { "--cached" } else { "--force" });
    c
}
fn slauth_probe(target: &Target, options: &Options) -> Result<Probe, String> {
    let output = call(slauth_command(target, true), options, false)?;
    if output.code != 0 {
        return Ok(if auth_error(&output) {
            Probe::Missing
        } else {
            Probe::Failed("SLAuth cache check failed; inspect Atlas installation and credential-store permissions")
        });
    }
    let Target::Slauth { audience, .. } = target else {
        unreachable!()
    };
    let text = std::str::from_utf8(&output.stdout)
        .map_err(|_| "SLAuth token format invalid")?
        .trim();
    let parts: Vec<_> = text.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|s| s.is_empty()) {
        return Ok(Probe::Failed("SLAuth returned an invalid JWT"));
    }
    let Ok(payload) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[1]) else {
        return Ok(Probe::Failed("SLAuth JWT payload invalid"));
    };
    let Ok(claims) = serde_json::from_slice::<Value>(&payload) else {
        return Ok(Probe::Failed("SLAuth JWT claims invalid"));
    };
    let matches = match &claims["aud"] {
        Value::String(s) => s == audience,
        Value::Array(a) => a.iter().any(|v| v.as_str() == Some(audience)),
        _ => false,
    };
    if !matches {
        return Ok(Probe::Failed(
            "SLAuth cached token does not match the requested audience",
        ));
    }
    let Some(expiry) = claims["exp"].as_u64() else {
        return Ok(Probe::Failed("SLAuth JWT expiry missing or invalid"));
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "System clock precedes Unix epoch")?
        .as_secs();
    if claims
        .get("nbf")
        .is_some_and(|v| v.as_u64().is_none_or(|n| n > now))
    {
        return Ok(Probe::Failed(
            "SLAuth token is not yet valid or has invalid nbf",
        ));
    }
    if expiry <= now {
        return Ok(Probe::Missing);
    }
    Ok(Probe::Valid(Evidence {
        remaining: Some(expiry - now),
        account: None,
    }))
}
fn probe(target: &Target, options: &Options) -> Result<Probe, String> {
    match target {
        Target::Bitbucket { account } => {
            let output = bb_status(target, options)?;
            let cached = bitbucket(&output, account.as_deref());
            if let Probe::Valid(evidence) = &cached {
                if enough(evidence, options) {
                    // workspace list has no account override. Refuse a named
                    // inactive account rather than proving the wrong identity.
                    let active = regex::Regex::new(r"(?m)^\s*✓?\s*([A-Za-z0-9._@-]+)\s+\(active\)")
                        .expect("static regex");
                    let text = String::from_utf8_lossy(&output.stdout);
                    if account.as_ref().is_some_and(|selected| {
                        active.captures(&text).is_none_or(|c| &c[1] != selected)
                    }) {
                        return Ok(Probe::Failed("Selected Bitbucket account is inactive; use bb auth switch before preflight"));
                    }
                    let proof = call(
                        command("bb", &["workspace", "list", "--json"]),
                        options,
                        false,
                    )?;
                    if proof.code != 0 {
                        return Ok(if auth_error(&proof) {
                            evidence
                                .account
                                .clone()
                                .map(Probe::MissingForAccount)
                                .unwrap_or(Probe::Missing)
                        } else {
                            Probe::Failed("Bitbucket API proof failed; inspect connectivity or workspace permissions")
                        });
                    }
                    if !serde_json::from_slice::<Value>(&proof.stdout).is_ok_and(|v| v.is_array()) {
                        return Ok(Probe::Failed(
                            "Bitbucket API proof did not return its documented JSON array",
                        ));
                    }
                }
            }
            Ok(cached)
        }
        Target::Twg {} => twig_probe(options),
        Target::Slauth { .. } => slauth_probe(target, options),
    }
}
fn enough(evidence: &Evidence, options: &Options) -> bool {
    evidence
        .remaining
        .is_none_or(|s| s > 0 && s >= options.min_valid_for)
}
fn ready(target: &Target, evidence: Evidence, refreshed: bool) -> Value {
    let checked_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    json!({"target":target.label(), "status":if refreshed {"refreshed"} else {"valid"}, "remaining_seconds":evidence.remaining, "account":evidence.account,
        "checked_at_unix_seconds":checked_at, "expires_at_unix_seconds":evidence.remaining.map(|s|checked_at.saturating_add(s)),
        "note":if evidence.remaining.is_none() {Some("Access verified now; provider does not expose credential expiry. No unattended-duration guarantee.")} else {None}})
}
fn blocked(target: &Target, reason: &str, action: &str) -> Value {
    json!({"target":target.label(),"status":"blocked","reason":reason,"action":action,"remaining_seconds":null})
}
fn refresh(
    target: &Target,
    options: &Options,
    evidence: Option<&Evidence>,
) -> Result<bool, String> {
    match target {
        Target::Bitbucket { account } => {
            // Token retrieval performs the provider's supported silent OAuth refresh.
            let mut c = command("bb", &["auth", "token"]);
            if let Some(account) = account {
                c.args(["--account", account]);
            }
            let output = call(c, options, false)?;
            if output.code != 0 && !auth_error(&output) {
                return Err("Bitbucket silent refresh failed with a non-auth error".into());
            }
            if let Probe::Valid(e) = probe(target, options)? {
                if enough(&e, options) {
                    return Ok(true);
                }
            }
            if options.non_interactive {
                return Ok(false);
            }
            let selected = account
                .as_ref()
                .or_else(|| evidence.and_then(|e| e.account.as_ref()));
            let mut login = command("bb", &["auth", "login"]);
            if let Some(account) = selected {
                login.args(["--account", account]);
            }
            writeln!(std::io::stderr(), "Bitbucket requires browser login; complete the browser prompt (bounded by --login-timeout).")
                .map_err(|_| "Cannot display authentication requirement")?;
            Ok(call(login, options, true)?.code == 0)
        }
        Target::Twg {} => {
            // Site/account selection and secret entry remain owned by TWG.
            // Its login requires a controlling terminal, so do not launch a
            // misleading prompt in our pipe-based supervisor.
            let output = call(twg_command(&["auth", "refresh", "--force"]), options, false)?;
            if output.code != 0 {
                return if auth_error(&output) {
                    Ok(false)
                } else {
                    Err("TWG silent refresh failed; use twg auth refresh to inspect provider diagnostics".into())
                };
            }
            let value: Value = serde_json::from_slice(&output.stdout)
                .map_err(|_| "TWG refresh response is not JSON")?;
            Ok(value
                .pointer("/data/status")
                .and_then(Value::as_str)
                .is_some_and(|s| ["refreshed", "fresh", "not_due", "skipped"].contains(&s)))
        }
        Target::Slauth { .. } => {
            if options.non_interactive {
                return Ok(false);
            }
            writeln!(std::io::stderr(), "SLAuth refresh may require browser/MFA; complete any prompt before the login deadline.")
                .map_err(|_| "Cannot display authentication requirement")?;
            let output = call(slauth_command(target, false), options, true)?;
            if output.code != 0 {
                let diagnostics = format!(
                    "{} {}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                )
                .to_ascii_lowercase();
                if [
                    "permission denied",
                    "access denied",
                    "forbidden",
                    "not a member",
                    "group membership",
                    "insufficient permissions",
                ]
                .iter()
                .any(|s| diagnostics.contains(s))
                {
                    return Err("SLAuth issuance denied by service or group policy; verify access rather than repeating MFA".into());
                }
            }
            if output.code != 0
                && !auth_error(&output)
                && ![
                    "mfa required",
                    "login required",
                    "authorization pending",
                    "authentication required",
                ]
                .iter()
                .any(|s| {
                    String::from_utf8_lossy(&output.stderr)
                        .to_ascii_lowercase()
                        .contains(s)
                        || String::from_utf8_lossy(&output.stdout)
                            .to_ascii_lowercase()
                            .contains(s)
                })
            {
                return Err(format!("SLAuth issuance failed (exit {}); inspect Atlas login and connectivity; cache rechecked without reissuing", output.code));
            }
            Ok(output.code == 0)
        }
    }
}
pub(crate) fn check(target: &Target, options: &Options) -> Value {
    let run = || -> Result<Value, String> {
        let initial = probe(target, options)?;
        let evidence = match initial {
            Probe::Valid(e) if enough(&e, options) => return Ok(ready(target, e, false)),
            Probe::Valid(e) => Some(e),
            Probe::Missing => None,
            Probe::MissingForAccount(account) => Some(Evidence {
                remaining: Some(0),
                account: Some(account),
            }),
            Probe::Failed(reason) => return Err(reason.into()),
        };
        let action = match target {
            Target::Bitbucket { .. } => "Complete bb auth login for the selected account, then rerun preflight",
            Target::Twg {} => "Run twg login in a terminal, then rerun preflight; TWG Bitbucket uses the separate bb fallback",
            Target::Slauth { .. } => "Rerun without --check/--non-interactive and complete Atlas browser/MFA; SLAuth TTL is at most one hour",
        };
        if options.check {
            return Ok(blocked(target, "Missing credential or insufficient remaining validity; check-only mode made no explicit refresh", action));
        }
        if options.non_interactive && matches!(target, Target::Slauth { .. }) {
            return Ok(blocked(
                target,
                "SLAuth issuance disabled in noninteractive mode",
                action,
            ));
        }
        let refreshed = match refresh(target, options, evidence.as_ref()) {
            Ok(refreshed) => refreshed,
            Err(error) => {
                // Issuance can finish writing its normal cache just before a
                // timeout or provider error. Check once; never repeat issuance.
                if !crate::runtime::cancelled() {
                    if let Ok(Probe::Valid(cached)) = probe(target, options) {
                        if enough(&cached, options) {
                            return Ok(ready(target, cached, true));
                        }
                    }
                }
                return Err(error);
            }
        };
        // A successful issuance is not proof. Recheck the provider's normal cache.
        match probe(target, options)? {
            Probe::Valid(e) if enough(&e, options) => Ok(ready(target, e, true)),
            Probe::Valid(e) => Ok(
                json!({"target":target.label(),"status":"blocked","reason":"Refreshed token cannot meet the requested minimum lifetime; provider TTL policy applies","remaining_seconds":e.remaining,"action":"Choose a shorter --min-valid-for; future refresh/MFA may still be required"}),
            ),
            Probe::Missing | Probe::MissingForAccount(_) => Ok(blocked(
                target,
                if refreshed {
                    "Credential still missing after one refresh; no retry loop"
                } else {
                    "Provider could not refresh without additional human authentication"
                },
                action,
            )),
            Probe::Failed(reason) => Err(reason.into()),
        }
    };
    match run() {
        Ok(value) => value,
        Err(reason) => {
            json!({"target":target.label(),"status":if crate::runtime::cancelled() {"interrupted"} else {"failed"},"reason":reason,"remaining_seconds":null})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn durations_fail_closed_and_support_provider_formats() {
        for (text, expected) in [
            ("1h 59m", 7140),
            ("23 min", 1380),
            ("2 hours 3 minutes 4 seconds", 7384),
            ("30s", 30),
        ] {
            assert_eq!(duration_seconds(text), Some(expected));
        }
        for text in ["", "a while", "3m secret", "18446744073709551615h", "-3m"] {
            assert_eq!(duration_seconds(text), None);
        }
    }
}
