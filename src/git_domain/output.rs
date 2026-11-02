use super::display;
use std::path::Path;

pub(super) fn recovery_command(path: &Path, reference: &str) -> String {
    format!(
        "git -C {} branch recovered-work {}",
        crate::presentation::shell_arg(path.as_os_str()),
        crate::presentation::shell_arg(reference.as_ref())
    )
}

pub(super) fn remote_access_guidance(
    path: &Path,
    detail: &str,
    remote: &str,
    branch: Option<&str>,
) -> Option<serde_json::Value> {
    if !(detail.starts_with("Error [resolve remote target]:")
        || detail.starts_with("Error [fetch]:"))
        || !super::git::remote_access_error(detail)
    {
        return None;
    }
    let quote = crate::presentation::shell_arg;
    let command = format!(
        "git -C {} ls-remote --symref {} HEAD",
        quote(path.as_os_str()),
        quote(remote.as_ref())
    );
    let mut retry = vec![
        "workctl".into(),
        "git".into(),
        "reset".into(),
        "--remote".into(),
        std::ffi::OsString::from(remote),
    ];
    if let Some(branch) = branch {
        retry.extend(["--branch".into(), branch.into()]);
    }
    retry.push(path.as_os_str().to_owned());
    Some(
        serde_json::json!({"action":"verify_remote_access", "destructive":false,
        "command":command, "retry_command":retry.iter().map(|arg|quote(arg)).collect::<Vec<_>>().join(" ")}),
    )
}

pub(super) fn short(path: &Path) -> String {
    if let Some(home) = std::env::var_os("HOME") {
        if let Ok(relative) = path.strip_prefix(home) {
            return format!("~/{}", display(relative));
        }
    }
    display(path)
}
pub(super) fn wrap(text: &str) -> String {
    let width = std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(80)
        .saturating_sub(4)
        .max(20);
    let mut result = String::from("    ");
    let mut length = 0;
    for word in text.split_whitespace() {
        let size = word.chars().count();
        if length != 0 && length + 1 + size > width {
            result.push_str("\n    ");
            length = 0;
        }
        if length != 0 {
            result.push(' ');
            length += 1;
        }
        result.push_str(word);
        length += size;
    }
    result
}

/// Keep the primary error readable without dumping a potentially enormous fetch
/// transcript. The complete redacted diagnostic remains in JSON and the log.
pub(super) fn failure(detail: &str) -> String {
    // The full invocation belongs in diagnostics. Lead with its actual failure,
    // so a long refspec cannot consume the summary before the cause appears.
    let concise = detail.split_once("]: git ").and_then(|(stage, command)| {
        let (_, result) = command.split_once(" failed (exit ")?;
        let (_, error) = result.split_once("): ")?;
        Some(format!("{stage}]: {error}"))
    });
    let detail = concise.as_deref().unwrap_or(detail);
    let lines: Vec<_> = detail
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let mut shortened = lines.len() > 3;
    let mut summary = Vec::new();
    // Fetch can print many successful ref updates before its actual error.
    // Keep that error visible even when the rest of the transcript is shortened.
    let errors: Vec<_> = lines
        .iter()
        .skip(1)
        .copied()
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with("error:")
                || line.starts_with("fatal:")
                || line.starts_with("remote: error:")
        })
        .collect();
    let selected: Vec<_> = if errors.is_empty() {
        lines.iter().take(3).copied().collect()
    } else {
        lines
            .first()
            .copied()
            .into_iter()
            .chain(errors.into_iter().take(2))
            .collect()
    };
    for line in selected {
        let safe = display(line.trim_end_matches('\r'));
        let mut chars = safe.chars();
        let mut line: String = chars.by_ref().take(400).collect();
        if chars.next().is_some() {
            line.push('…');
            shortened = true;
        }
        summary.push(wrap(&line));
    }
    if shortened {
        summary.push("    … full diagnostics in log".into());
    }
    summary.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_after_fetch_progress_is_visible_and_terminal_controls_are_escaped() {
        let detail = format!(
            "Error [fetch]: From remote\n{}\nfatal: actual failure\x1b[31m\n",
            "updated branch\n".repeat(100)
        );
        let rendered = failure(&detail);
        assert!(rendered.contains("fatal: actual failure\\u{1b}[31m"));
        assert!(!rendered.contains("\\u{a}"));
        assert!(!rendered.contains('\x1b'));
        assert!(rendered.len() < 300);
    }

    #[test]
    fn a_single_huge_line_is_bounded_without_splitting_unicode() {
        let rendered = failure(&"🦀".repeat(10000));
        assert!(rendered.chars().count() < 500);
        assert!(rendered.contains("full diagnostics in log"));
    }

    #[test]
    fn long_git_commands_do_not_hide_the_error() {
        let detail = format!(
            "Error [fetch]: git fetch origin {} failed (exit 1): fatal: connection lost",
            "branch".repeat(1000)
        );
        assert_eq!(
            failure(&detail),
            "    Error [fetch]: fatal: connection lost"
        );
    }
}
