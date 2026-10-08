use super::display;
use std::path::Path;
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
