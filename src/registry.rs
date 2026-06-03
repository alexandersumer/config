use crate::error::Result;
use regex::Regex;
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

const SKILL_FRONT_MATTER_KEYS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
];
const MIN_SKILL_BODY_WORDS: usize = 40;
const MIN_SKILL_BODY_LINES: usize = 3;
const REQUIRED_CUSTOM_SKILL_NAMES: &[&str] = &[
    "address-comments",
    "architecture-review",
    "architecture-review-solo",
    "clean-up-feature-flag",
    "create-chat-plan",
    "create-plan",
    "describe-branch",
    "describe-diff",
    "design-review",
    "design-review-solo",
    "diagnose",
    "execute-plan",
    "fix-failures",
    "git-publish",
    "git-publish-to-origin",
    "grill-me",
    "prove-check",
    "real-e2e",
    "reconcile-plan",
    "resolve-conflict",
    "review",
    "review-solo",
    "strengthen-tests",
    "strengthen-tests-solo",
    "study-code-atlassian",
    "study-code-oss",
    "surgical-edit",
    "sync-main",
    "understand-system",
    "verify-and-fix",
    "write-up",
];

fn parse_front_matter(path: &Path) -> Result<Mapping> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("{}: cannot read skill file: {err}", path.display()))?;
    let lines: Vec<&str> = text.lines().collect();
    if lines.first() != Some(&"---") {
        return Err(format!(
            "{}: missing opening front matter delimiter",
            path.display()
        ));
    }
    let closing_index = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (*line == "---").then_some(index))
        .ok_or_else(|| format!("{}: missing closing front matter delimiter", path.display()))?;
    let body = lines[closing_index + 1..].join("\n");
    let yaml_text = lines[1..closing_index].join("\n");
    let value: Value = serde_yaml::from_str(&yaml_text)
        .map_err(|err| format!("{}: invalid front matter YAML: {err}", path.display()))?;
    let metadata = value
        .as_mapping()
        .cloned()
        .ok_or_else(|| format!("{}: front matter must be a mapping", path.display()))?;
    validate_skill_body(path, &metadata, &body)?;
    Ok(metadata)
}

fn validate_skill_body(path: &Path, metadata: &Mapping, body: &str) -> Result<()> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Err(format!("{}: skill body must not be empty", path.display()));
    }

    let meaningful_lines = trimmed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("<!--") && !line.starts_with("-->"))
        .count();
    if meaningful_lines < MIN_SKILL_BODY_LINES {
        return Err(format!(
            "{}: skill body must contain at least {MIN_SKILL_BODY_LINES} meaningful instruction lines",
            path.display()
        ));
    }

    let words: Vec<String> = trimmed
        .split(|ch: char| !ch.is_alphanumeric() && ch != '-')
        .filter(|word| word.chars().any(char::is_alphabetic))
        .map(str::to_lowercase)
        .collect();
    let word_count = words.len();
    if word_count < MIN_SKILL_BODY_WORDS {
        return Err(format!(
            "{}: skill body looks too thin ({word_count} words); expected at least {MIN_SKILL_BODY_WORDS} words of real instructions",
            path.display()
        ));
    }

    let normalized_body = normalize_for_body_comparison(trimmed);
    for key in ["name", "description"] {
        if let Some(value) = get(metadata, key).and_then(Value::as_str) {
            let normalized_value = normalize_for_body_comparison(value);
            if !normalized_value.is_empty() && normalized_body == normalized_value {
                return Err(format!(
                    "{}: skill body must not merely repeat front matter {key}",
                    path.display()
                ));
            }
        }
    }

    let lower_body = normalized_body.to_lowercase();
    let placeholder_markers = [
        "todo",
        "tbd",
        "placeholder",
        "coming soon",
        "to be written",
        "intentionally blank",
    ];
    let placeholder_word_count = words
        .iter()
        .filter(|word| placeholder_markers.contains(&word.as_str()))
        .count();
    if placeholder_markers
        .iter()
        .any(|marker| lower_body == *marker)
        || placeholder_word_count * 2 >= word_count
    {
        return Err(format!(
            "{}: skill body must contain real instructions, not a placeholder",
            path.display()
        ));
    }

    Ok(())
}

fn normalize_for_body_comparison(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn validate_skill_name(value: Option<&Value>, context: &str) -> Result<String> {
    let name = value
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{context}: name must be a non-empty string"))?;
    if name.is_empty() {
        return Err(format!("{context}: name must be a non-empty string"));
    }
    if !skill_name_regex().is_match(name) {
        return Err(format!(
            "{context}: skill name must be kebab-case: {name:?}"
        ));
    }
    Ok(name.to_owned())
}

fn skill_name_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").expect("valid regex"))
}

fn validate_description(value: Option<&Value>, context: &str) -> Result<String> {
    let description = value
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{context}: description must be a non-empty string"))?;
    if description.trim().is_empty() {
        return Err(format!("{context}: description must be a non-empty string"));
    }
    if description.contains('\n') {
        return Err(format!("{context}: description must be a single line"));
    }
    Ok(description.to_owned())
}

pub(crate) fn string_key(key: &str) -> Value {
    Value::String(key.to_string())
}

pub(crate) fn get<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(string_key(key))
}

fn key_set(mapping: &Mapping) -> BTreeSet<String> {
    mapping
        .keys()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn require_known_keys(mapping: &Mapping, allowed: &[&str], context: &str) -> Result<()> {
    let allowed: BTreeSet<String> = allowed.iter().map(|key| key.to_string()).collect();
    let actual = key_set(mapping);
    let extra: Vec<String> = actual.difference(&allowed).cloned().collect();
    if !extra.is_empty() {
        return Err(format!("{context}: unsupported keys: {extra:?}"));
    }
    Ok(())
}

fn collect_skill_names(config_root: &Path) -> Result<HashSet<String>> {
    let skills_root = config_root.join(".agents/skills");
    if !skills_root.is_dir() {
        return Err(format!(
            "{}: canonical skills directory does not exist",
            skills_root.display()
        ));
    }
    let mut skill_files = Vec::new();
    for entry in fs::read_dir(&skills_root)
        .map_err(|err| format!("{}: cannot read directory: {err}", skills_root.display()))?
    {
        let entry = entry.map_err(|err| {
            format!(
                "{}: cannot read directory entry: {err}",
                skills_root.display()
            )
        })?;
        let path = entry.path();
        if !entry
            .file_type()
            .map_err(|err| format!("{}: cannot inspect file type: {err}", path.display()))?
            .is_dir()
        {
            continue;
        }
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let skill_file = path.join("SKILL.md");
        if skill_file.is_file() {
            skill_files.push(skill_file);
        }
    }
    skill_files.sort();

    let mut seen_names = HashSet::new();
    for skill_file in skill_files {
        let skill_parent = skill_file
            .parent()
            .ok_or_else(|| format!("{}: invalid skill path", skill_file.display()))?;
        let skill_dir = skill_parent
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("{}: invalid skill directory", skill_file.display()))?;
        let skill_dir_context = skill_parent.display().to_string();
        validate_skill_name(
            Some(&Value::String(skill_dir.to_owned())),
            &skill_dir_context,
        )?;
        let metadata = parse_front_matter(&skill_file)?;
        require_known_keys(
            &metadata,
            SKILL_FRONT_MATTER_KEYS,
            &skill_file.display().to_string(),
        )?;
        let name = validate_skill_name(get(&metadata, "name"), &skill_file.display().to_string())?;
        if name != skill_dir {
            return Err(format!(
                "{}: front matter name {name:?} does not match directory {skill_dir:?}",
                skill_file.display()
            ));
        }
        if !seen_names.insert(name.clone()) {
            return Err(format!(
                "{}: duplicate skill name: {name}",
                skill_file.display()
            ));
        }
        validate_description(
            get(&metadata, "description"),
            &skill_file.display().to_string(),
        )?;
    }

    if seen_names.is_empty() {
        return Err(format!("{}: no skills found", skills_root.display()));
    }
    require_required_custom_skills(&skills_root, &seen_names)?;
    Ok(seen_names)
}

fn require_required_custom_skills(skills_root: &Path, seen_names: &HashSet<String>) -> Result<()> {
    let missing: Vec<&str> = REQUIRED_CUSTOM_SKILL_NAMES
        .iter()
        .copied()
        .filter(|name| !seen_names.contains(*name))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{}: missing required custom skills: {}",
            skills_root.display(),
            missing.join(", ")
        ))
    }
}

pub(crate) fn validate_registry(config_root: &Path) -> Vec<String> {
    match collect_skill_names(config_root) {
        Ok(_) => Vec::new(),
        Err(err) => vec![err],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml_value(text: &str) -> Value {
        serde_yaml::from_str(text).expect("valid test YAML")
    }

    #[test]
    fn validate_skill_name_accepts_kebab_case_and_rejects_other_shapes() {
        assert_eq!(
            validate_skill_name(
                Some(&Value::String("clean-up-feature-flag".to_string())),
                "skill"
            )
            .expect("valid skill name"),
            "clean-up-feature-flag"
        );
        assert!(validate_skill_name(Some(&Value::String("CleanUp".to_string())), "skill").is_err());
        assert!(validate_skill_name(Some(&Value::String("".to_string())), "skill").is_err());
    }

    #[test]
    fn skill_front_matter_rejects_legacy_register_cmd_key() {
        let value = yaml_value(
            r#"
name: surgical-edit
description: Apply a narrow requested code change.
register_cmd: true
"#,
        );
        let metadata = value.as_mapping().expect("front matter mapping");

        let error = require_known_keys(metadata, SKILL_FRONT_MATTER_KEYS, "skill")
            .expect_err("register_cmd should not be accepted because Codex does not expose custom skills as slash commands");
        assert!(error.contains("register_cmd"));
    }

    #[test]
    fn validate_skill_body_rejects_non_instructional_stubs() {
        let value = yaml_value(
            r#"
name: surgical-edit
description: Apply a narrow requested code change.
"#,
        );
        let metadata = value.as_mapping().expect("front matter mapping");
        let path = Path::new(".agents/skills/surgical-edit/SKILL.md");

        assert!(validate_skill_body(path, metadata, "").is_err());
        assert!(validate_skill_body(path, metadata, "TODO").is_err());
        assert!(validate_skill_body(path, metadata, "Use conversation context.").is_err());
        assert!(
            validate_skill_body(path, metadata, "Apply a narrow requested code change.").is_err()
        );
    }

    #[test]
    fn validate_skill_body_accepts_concise_real_instructions() {
        let value = yaml_value(
            r#"
name: surgical-edit
description: Apply a narrow requested code change.
"#,
        );
        let metadata = value.as_mapping().expect("front matter mapping");
        let path = Path::new(".agents/skills/surgical-edit/SKILL.md");
        let body = r#"
Read the relevant files before editing, then make the smallest correct change that matches the existing naming, layering, error handling, and tests.
Do not add dependencies, abstractions, broad refactors, or explanatory comments unless the requested change requires them.
If the request remains ambiguous after reading context, ask one focused question and stop instead of guessing.
"#;

        validate_skill_body(path, metadata, body).expect("real instructions are valid");
    }
}
