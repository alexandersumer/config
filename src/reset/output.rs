//! Line-oriented output: terse successes, immediate actionable failures.
use super::{
    discover::{Repo, Scope},
    display,
};
use std::io::{self, IsTerminal};
use std::path::Path;

pub(super) fn quantity(n: usize, noun: &str) -> String {
    let suffix = if n == 1 { "" } else { "s" };
    if noun == "repository" && n != 1 {
        format!("{n} repositories")
    } else {
        format!("{n} {noun}{suffix}")
    }
}
pub(super) fn color(text: &str, code: u8) -> String {
    if io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var_os("TERM").is_none_or(|v| v != "dumb")
    {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.into()
    }
}
pub(super) fn short(path: &Path) -> String {
    if let Some(home) = std::env::var_os("HOME") {
        if let Ok(relative) = path.strip_prefix(home) {
            return format!("~/{}", display(relative));
        }
    }
    display(path)
}
pub(super) fn label(repo: &Repo, scopes: &[Scope]) -> String {
    if scopes.len() == 1 {
        if let Ok(relative) = repo.path.strip_prefix(&scopes[0].path) {
            if !relative.as_os_str().is_empty() {
                return display(relative);
            }
        }
        if let Some(name) = repo.path.file_name() {
            return display(name);
        }
    }
    short(&repo.path)
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_have_correct_grammar() {
        assert_eq!(quantity(1, "repository"), "1 repository");
        assert_eq!(quantity(2, "repository"), "2 repositories");
        assert_eq!(quantity(1, "worker"), "1 worker");
    }
}
