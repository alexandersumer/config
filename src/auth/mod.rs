//! Auth readiness is deterministic and scoped to declared credential targets.
mod config;
mod process;
mod providers;
pub(crate) use config::Options;
use serde_json::{json, Value};
use std::ffi::OsString;
use std::io::Write;

fn result(options: &Options) -> Value {
    let execute = || -> Result<Value, String> {
        let targets = config::targets(options)?;
        let _lock = process::Lock::acquire(options.timeout)?;
        let mut results = Vec::new();
        for target in targets {
            if crate::runtime::cancelled() {
                break;
            }
            results.push(providers::check(&target, options));
        }
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
            json!({"schema_version":1,"operation":"auth.preflight","profile":options.profile,"status":if crate::runtime::cancelled() {"interrupted"} else if ready {"completed"} else {"blocked"}, "check_only":options.check,"min_valid_for_seconds":options.min_valid_for,"targets":results,
            "duration_guaranteed":false,"note":"Readiness is checked now. OAuth revocation, provider policy, unknown expiry, and future MFA can shorten access. Only workctl refreshes share this coordination lock."}),
        )
    };
    match execute() {
        Ok(value) => value,
        Err(reason) => {
            json!({"schema_version":1,"operation":"auth.preflight","status":if crate::runtime::cancelled() {"interrupted"} else {"failed"},"error":reason})
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
fn render(result: &Value, stderr: bool) -> std::io::Result<()> {
    let mut output: Box<dyn Write> = if stderr {
        Box::new(std::io::stderr())
    } else {
        Box::new(std::io::stdout())
    };
    writeln!(
        output,
        "Authentication preflight: {}",
        result["status"].as_str().unwrap_or("failed")
    )?;
    if let Some(error) = result["error"].as_str() {
        writeln!(output, "  {}", crate::presentation::safe_text(error))?;
    }
    for target in result["targets"].as_array().into_iter().flatten() {
        let ttl = target["remaining_seconds"]
            .as_u64()
            .map(|s| {
                format!(
                    "{} remaining",
                    crate::presentation::duration(std::time::Duration::from_secs(s))
                )
            })
            .unwrap_or_else(|| "expiry unavailable".into());
        writeln!(
            output,
            "  {}  {}  ({ttl})",
            target["target"].as_str().unwrap_or("unknown"),
            target["status"].as_str().unwrap_or("failed")
        )?;
        for key in ["reason", "action", "note"] {
            if let Some(text) = target[key].as_str() {
                writeln!(output, "    {}", crate::presentation::safe_text(text))?;
            }
        }
    }
    if let Some(note) = result["note"].as_str() {
        writeln!(output, "\n{note}")?;
    }
    output.flush()
}
pub(crate) fn run(options: Options, json_output: bool, child: Option<Vec<OsString>>) -> u8 {
    if json_output && child.is_some() {
        let _ = crate::presentation::json(
            &json!({"schema_version":1,"operation":"auth.exec","status":"failed","error":"--json is supported by auth preflight only; auth exec preserves child stdout"}),
        );
        return 2;
    }
    let signals = match crate::runtime::Signals::install() {
        Ok(signals) => signals,
        Err(_) => {
            let value = json!({"schema_version":1,"operation":"auth.preflight","status":"failed","error":"Cannot install cancellation handlers"});
            if json_output {
                let _ = crate::presentation::json(&value);
            } else {
                let _ = render(&value, child.is_some());
            }
            return 1;
        }
    };
    let outcome = result(&options);
    let status = code(&outcome);
    let output_status = if json_output {
        crate::presentation::json(&outcome)
    } else {
        u8::from(render(&outcome, child.is_some()).is_err())
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
