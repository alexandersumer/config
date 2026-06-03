use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(crate) fn validate_managed_configs(config_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    errors.extend(validate_ghostty_config(config_root));
    errors
}

fn validate_ghostty_config(config_root: &Path) -> Vec<String> {
    let path = config_root.join("ghostty/config");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            return vec![format!(
                "{}: cannot read Ghostty config: {err}",
                path.display()
            )];
        }
    };

    let assignments = ghostty_assignments(&text);
    let mut final_values = BTreeMap::new();
    let mut errors = Vec::new();

    for assignment in &assignments {
        final_values.insert(assignment.key.as_str(), assignment.value.as_str());

        for risky in RISKY_GHOSTTY_SETTINGS {
            if assignment.key == risky.key && assignment.value == risky.value {
                errors.push(format!(
                    "{}:{}: Ghostty config must not set `{}` to `{}`; {}",
                    path.display(),
                    assignment.line_number,
                    risky.key,
                    risky.value,
                    risky.reason
                ));
            }
        }
    }

    for required in REQUIRED_GHOSTTY_SETTINGS {
        match final_values.get(required.key).copied() {
            Some(value) if value == required.value => {}
            Some(value) => errors.push(format!(
                "{}: Ghostty config must set `{}` to `{}`; found `{}`. {}",
                path.display(),
                required.key,
                required.value,
                value,
                required.reason
            )),
            None => errors.push(format!(
                "{}: Ghostty config must set `{}` to `{}`. {}",
                path.display(),
                required.key,
                required.value,
                required.reason
            )),
        }
    }

    errors
}

fn ghostty_assignments(text: &str) -> Vec<GhosttyAssignment> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }

            let (key, value) = line.split_once('=')?;
            Some(GhosttyAssignment {
                line_number: index + 1,
                key: key.trim().to_string(),
                value: normalize_ghostty_value(value),
            })
        })
        .collect()
}

fn normalize_ghostty_value(value: &str) -> String {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
        .to_string()
}

struct GhosttyAssignment {
    line_number: usize,
    key: String,
    value: String,
}

struct GhosttySetting {
    key: &'static str,
    value: &'static str,
    reason: &'static str,
}

const REQUIRED_GHOSTTY_SETTINGS: &[GhosttySetting] = &[
    GhosttySetting {
        key: "macos-titlebar-style",
        value: "transparent",
        reason: "transparent keeps native macOS tab behavior instead of Ghostty's fragile custom titlebar tabs.",
    },
    GhosttySetting {
        key: "macos-non-native-fullscreen",
        value: "false",
        reason: "native fullscreen avoids adding a separate fullscreen behavior while isolating tab visibility.",
    },
    GhosttySetting {
        key: "window-save-state",
        value: "never",
        reason: "saved tab/window restoration is part of the known disappearing-tab failure path.",
    },
    GhosttySetting {
        key: "window-show-tab-bar",
        value: "always",
        reason: "always showing the tab bar avoids auto show/hide transitions while debugging tab visibility.",
    },
];

const RISKY_GHOSTTY_SETTINGS: &[GhosttySetting] = &[
    GhosttySetting {
        key: "macos-titlebar-style",
        value: "tabs",
        reason: "custom titlebar tabs are the primary known macOS disappearing-tab trigger.",
    },
    GhosttySetting {
        key: "macos-non-native-fullscreen",
        value: "true",
        reason: "non-native fullscreen was present in the old local risky config and adds another tab/window transition path.",
    },
    GhosttySetting {
        key: "window-save-state",
        value: "always",
        reason: "forced window restoration is part of the upstream disappearing-tab reproduction.",
    },
];
