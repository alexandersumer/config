//! Auth readiness is deterministic and scoped to declared credential targets.
mod config;
mod process;
mod providers;
pub(crate) use config::Options;
use serde_json::{json, Value};
use std::ffi::OsString;
use std::io::Write;

fn result(options: &Options, human: bool) -> Value {
    let execute = || -> Result<Value, String> {
        let targets = config::targets(options)?;
        if human {
            let mode = if options.check {
                "check only"
            } else if options.reuse_valid {
                "reuse valid credentials"
            } else {
                "renew all credentials"
            };
            writeln!(
                std::io::stderr(),
                "Authentication preflight · profile {} · {mode}",
                inline(
                    &std::iter::once(options.profile.as_str())
                        .chain(options.include_profiles.iter().map(String::as_str))
                        .collect::<Vec<_>>()
                        .join(" + ")
                )
            )
            .map_err(|_| "Cannot display authentication progress")?;
        }
        let _lock =
            process::Lock::acquire(options.coordination_timeout.unwrap_or(options.timeout))?;
        let session = process::Session::default();
        let total = targets.len();
        let mut queues: [Vec<_>; 4] = std::array::from_fn(|_| Vec::new());
        for (index, target) in targets.into_iter().enumerate() {
            let provider = match &target {
                config::Target::Bitbucket { .. } => 0,
                config::Target::Twg {} | config::Target::TwgBitbucket {} => 1,
                config::Target::Kerberos { .. } => 3,
                config::Target::Slauth { .. } => 2,
            };
            queues[provider].push((index, target));
        }
        let mut indexed = std::thread::scope(|scope| -> Result<Vec<_>, String> {
            let mut workers = Vec::new();
            let mut failure = None;
            for queue in queues.into_iter().filter(|queue| !queue.is_empty()) {
                let session = &session;
                let worker = std::thread::Builder::new().spawn_scoped(scope, move || {
                    // Catch worker failures here so siblings stop before join waits.
                    let work = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let mut completed = Vec::new();
                        for (index, target) in queue {
                            if crate::runtime::cancelled() {
                                break;
                            }
                            if human {
                                session.message(&format!(
                                    "  [{}/{}] {} {}{}…",
                                    index + 1,
                                    total,
                                    if options.check {
                                        "Checking"
                                    } else if options.reuse_valid {
                                        "Preparing"
                                    } else {
                                        "Renewing"
                                    },
                                    target.display_name(),
                                    target
                                        .scope()
                                        .map(|s| format!(" ({s})"))
                                        .unwrap_or_default()
                                ))?;
                            }
                            let mut checked = providers::check(&target, options, session);
                            if human {
                                checked["display_name"] = json!(target.display_name());
                                checked["scope"] = json!(target.scope());
                            }
                            completed.push((index, checked));
                        }
                        Ok::<_, String>(completed)
                    }));
                    match work {
                        Ok(result) => result,
                        Err(_) => {
                            session.fail_cleanup();
                            Err("Authentication worker failed; process cleanup is uncertain".into())
                        }
                    }
                });
                match worker {
                    Ok(worker) => workers.push(worker),
                    Err(_) => {
                        session.fail_cleanup();
                        failure = Some("Cannot start authentication worker".into());
                        break;
                    }
                }
            }
            let mut completed = Vec::new();
            // Always join every worker before releasing coordination or exec.
            for worker in workers {
                match worker.join() {
                    Ok(Ok(results)) => completed.extend(results),
                    Ok(Err(error)) => {
                        failure = Some(error);
                    }
                    Err(_) => {
                        session.fail_cleanup();
                        failure = Some("Authentication worker failed".into());
                    }
                }
            }
            if session.cleanup_failed() {
                return Err(
                    "Cannot confirm authentication worker cleanup; command will not launch".into(),
                );
            }
            if let Some(error) = failure {
                return Err(error);
            }
            Ok(completed)
        })?;
        indexed.sort_by_key(|(index, _)| *index);
        let mut results: Vec<_> = indexed.into_iter().map(|(_, value)| value).collect();
        // Earlier targets may age while a later MFA flow waits. Evaluate
        // readiness at handoff, not only when each individual check started.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "System clock precedes Unix epoch")?
            .as_secs();
        for target in &mut results {
            if let Some(expiry) = target["expires_at_unix_seconds"].as_u64() {
                let remaining = expiry.saturating_sub(now);
                target["remaining_seconds"] = json!(remaining);
                if remaining == 0 || remaining < options.min_valid_for {
                    target["status"] = json!("blocked");
                    target["reason"] = json!(
                        "Credential aged below minimum validity while another target was checked"
                    );
                    target["action"] = json!("Rerun preflight or choose a shorter --min-valid-for; no unbounded refresh loop");
                }
            }
        }
        let ready = results
            .iter()
            .all(|v| matches!(v["status"].as_str(), Some("valid" | "refreshed")));
        Ok(
            json!({"schema_version":1,"operation":"auth.preflight","profile":options.profile,"included_profiles":options.include_profiles,"status":if crate::runtime::cancelled() {"interrupted"} else if ready {"completed"} else {"blocked"}, "check_only":options.check,"min_valid_for_seconds":options.min_valid_for,"targets":results,
            "duration_guaranteed":false,"note":"Readiness is checked now. OAuth revocation, provider policy, unknown expiry, and future MFA can shorten access. Only workctl refreshes share this coordination lock."}),
        )
    };
    match execute() {
        Ok(value) => value,
        Err(reason) => {
            json!({"schema_version":1,"operation":"auth.preflight","status":if crate::runtime::interrupted() {"interrupted"} else {"failed"},"error":reason})
        }
    }
}
fn code(result: &Value) -> u8 {
    match result["status"].as_str() {
        Some("completed") => 0,
        Some("interrupted") => 130,
        _ => 1,
    }
}
fn inline(text: &str) -> String {
    crate::presentation::safe_text(text).replace(['\n', '\t'], " ")
}
fn lifetime(seconds: u64) -> String {
    let mut parts = Vec::new();
    for (value, unit) in [
        (seconds / 3600, "h"),
        (seconds % 3600 / 60, "m"),
        (seconds % 60, "s"),
    ] {
        if value > 0 {
            parts.push(format!("{value}{unit}"));
        }
    }
    if parts.is_empty() {
        "0s".into()
    } else {
        parts.join(" ")
    }
}
fn write_detail(output: &mut dyn Write, label: &str, text: &str) -> std::io::Result<()> {
    let width = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(80)
        .max(20);
    let text = format!("{label}: {}", inline(text));
    let mut line = String::from("    ");
    for word in text.split_whitespace() {
        if line.len() > 4 && line.chars().count() + 1 + word.chars().count() > width {
            writeln!(output, "{line}")?;
            line = String::from("    ");
        }
        if line.len() > 4 {
            line.push(' ');
        }
        line.push_str(word);
    }
    writeln!(output, "{line}")
}
fn render(result: &Value, stderr: bool, elapsed: std::time::Duration) -> std::io::Result<()> {
    let mut output: Box<dyn Write> = if stderr {
        Box::new(std::io::stderr())
    } else {
        Box::new(std::io::stdout())
    };
    let state = result["status"].as_str().unwrap_or("failed");
    let outcome = match state {
        "completed" => "ready",
        other => other,
    };
    writeln!(
        output,
        "Authentication preflight: {}",
        crate::presentation::status(outcome, state != "completed", stderr)
    )?;
    if let Some(error) = result["error"].as_str() {
        writeln!(output, "  Error: {}", inline(error))?;
    }
    let targets: Vec<_> = result["targets"].as_array().into_iter().flatten().collect();
    let mut ready = 0;
    for target in &targets {
        let state = target["status"].as_str().unwrap_or("failed");
        let valid = matches!(state, "valid" | "refreshed");
        ready += usize::from(valid);
        let ttl = target["remaining_seconds"]
            .as_u64()
            .map(|s| format!("{} remaining", lifetime(s)));
        let detail = ttl.as_deref().unwrap_or(if valid {
            "expiry unknown"
        } else {
            "access not ready"
        });
        writeln!(
            output,
            "  {:<9}  {}  {detail}",
            inline(target["display_name"].as_str().unwrap_or("unknown")),
            crate::presentation::status(&format!("{state:<11}"), !valid, stderr)
        )?;
        if let Some(scope) = target["scope"].as_str() {
            write_detail(&mut output, "Scope", scope)?;
        }
        if let Some(account) = target["account"].as_str() {
            write_detail(&mut output, "Account", account)?;
        }
        for (key, label) in [("reason", "Reason"), ("action", "Action"), ("note", "Note")] {
            if let Some(text) = target[key].as_str() {
                write_detail(&mut output, label, text)?;
            }
        }
    }
    if !targets.is_empty() {
        writeln!(output, "\n  {ready}/{} credentials ready", targets.len())?;
    }
    writeln!(
        output,
        "  Elapsed {}",
        crate::presentation::duration(elapsed)
    )?;
    output.flush()
}
pub(crate) fn run(options: Options, json_output: bool, child: Option<Vec<OsString>>) -> u8 {
    if json_output && child.is_some() {
        let _ = crate::presentation::json(
            &json!({"schema_version":1,"operation":"auth.exec","status":"failed","error":"--json is supported by auth preflight only; auth exec preserves child stdout"}),
        );
        return 2;
    }
    let started = std::time::Instant::now();
    let signals = match crate::runtime::Signals::install() {
        Ok(signals) => signals,
        Err(_) => {
            let value = json!({"schema_version":1,"operation":"auth.preflight","status":"failed","error":"Cannot install cancellation handlers"});
            if json_output {
                let _ = crate::presentation::json(&value);
            } else {
                let _ = render(&value, child.is_some(), started.elapsed());
            }
            return 1;
        }
    };
    let outcome = result(&options, !json_output);
    let status = code(&outcome);
    let output_status = if json_output {
        crate::presentation::json(&outcome)
    } else {
        u8::from(render(&outcome, child.is_some(), started.elapsed()).is_err())
    };
    if output_status != 0 {
        return output_status;
    }
    if status != 0 {
        return status;
    }
    if let Some(args) = child {
        if crate::runtime::cancelled() {
            return 130;
        }
        // Release coordination and restore handlers before replacing ourselves.
        // exec preserves argv bytes, stdin/TTY, stdout, signals, and child exit code.
        drop(signals);
        use std::os::unix::process::CommandExt;
        let error = std::process::Command::new(&args[0]).args(&args[1..]).exec();
        let _ = writeln!(
            std::io::stderr(),
            "workctl: cannot launch command ({:?})",
            error.kind()
        );
        return if error.kind() == std::io::ErrorKind::NotFound {
            127
        } else {
            126
        };
    }
    0
}
