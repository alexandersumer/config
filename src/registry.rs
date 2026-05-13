use crate::error::Result;
use regex::Regex;
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const INPUT_KEYS: &[&str] = &["name", "label", "description", "type", "required"];
const SKILL_FRONT_MATTER_KEYS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
    "register_cmd",
];
const PROMPT_REQUIRED_KEYS: &[&str] = &["name", "description", "content_file"];
const PROMPT_KEYS: &[&str] = &["name", "description", "content_file", "inputs"];
const PROMPT_METADATA_KEYS: &[&str] = &["inputs"];
const CONTENT_FILE_PREFIX: &str = "prompts/";
const CONTENT_FILE_SUFFIX: &str = "/SKILL.md";

#[derive(Debug, Clone, PartialEq, Eq)]
struct Input {
    name: String,
    label: String,
    description: String,
    input_type: String,
    required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Prompt {
    name: String,
    description: String,
    content_file: String,
    inputs: Vec<Input>,
}

pub(crate) fn load_yaml_file(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("{}: cannot read file: {err}", path.display()))?;
    serde_yaml::from_str(&text).map_err(|err| format!("{}: invalid YAML: {err}", path.display()))
}

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
    if body.trim().is_empty() {
        return Err(format!("{}: skill body must not be empty", path.display()));
    }
    let yaml_text = lines[1..closing_index].join("\n");
    let value: Value = serde_yaml::from_str(&yaml_text)
        .map_err(|err| format!("{}: invalid front matter YAML: {err}", path.display()))?;
    value
        .as_mapping()
        .cloned()
        .ok_or_else(|| format!("{}: front matter must be a mapping", path.display()))
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

fn validate_input_name(value: Option<&Value>, context: &str) -> Result<String> {
    let name = value
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{context}: input name must be a non-empty string"))?;
    if name.is_empty() {
        return Err(format!("{context}: input name must be a non-empty string"));
    }
    if !input_name_regex().is_match(name) {
        return Err(format!(
            "{context}: input name must be lower_snake_case: {name:?}"
        ));
    }
    Ok(name.to_owned())
}

fn skill_name_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").expect("valid regex"))
}

fn input_name_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^[a-z][a-z0-9_]*$").expect("valid regex"))
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

fn require_keys(mapping: &Mapping, required: &[&str], context: &str) -> Result<()> {
    let actual = key_set(mapping);
    let missing: Vec<String> = required
        .iter()
        .filter(|key| !actual.contains(**key))
        .map(|key| key.to_string())
        .collect();
    if !missing.is_empty() {
        return Err(format!("{context}: missing keys: {missing:?}"));
    }
    Ok(())
}

fn normalize_inputs(value: Option<&Value>, context: &str) -> Result<Vec<Input>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = value
        .as_sequence()
        .ok_or_else(|| format!("{context}: inputs must be a list"))?;
    let mut normalized = Vec::new();
    let mut seen_names = HashSet::new();
    for (index, item) in items.iter().enumerate() {
        let item_context = format!("{context}: inputs[{index}]");
        let mapping = item
            .as_mapping()
            .ok_or_else(|| format!("{item_context}: input must be a mapping"))?;
        require_keys(mapping, INPUT_KEYS, &item_context)?;
        require_known_keys(mapping, INPUT_KEYS, &item_context)?;
        let name = validate_input_name(get(mapping, "name"), &item_context)?;
        if !seen_names.insert(name.clone()) {
            return Err(format!("{item_context}: duplicate input name: {name}"));
        }
        let label = get(mapping, "label")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{item_context}: label must be a non-empty string"))?;
        if label.trim().is_empty() {
            return Err(format!("{item_context}: label must be a non-empty string"));
        }
        let description = validate_description(get(mapping, "description"), &item_context)?;
        let input_type = get(mapping, "type")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{item_context}: only string inputs are currently supported"))?;
        if input_type != "string" {
            return Err(format!(
                "{item_context}: only string inputs are currently supported"
            ));
        }
        let required = get(mapping, "required")
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("{item_context}: required must be a boolean"))?;
        normalized.push(Input {
            name,
            label: label.to_string(),
            description,
            input_type: input_type.to_string(),
            required,
        });
    }
    Ok(normalized)
}

fn existing_prompt_order(config_root: &Path) -> Result<HashMap<String, usize>> {
    let config_path = config_root.join("rovodev/prompts.yml");
    if !config_path.exists() {
        return Ok(HashMap::new());
    }
    let config = load_yaml_file(&config_path)?;
    let Some(prompts) = config
        .as_mapping()
        .and_then(|mapping| get(mapping, "prompts"))
        .and_then(Value::as_sequence)
    else {
        return Ok(HashMap::new());
    };
    let mut order = HashMap::new();
    for (index, prompt) in prompts.iter().enumerate() {
        if let Some(name) = prompt
            .as_mapping()
            .and_then(|mapping| get(mapping, "name"))
            .and_then(Value::as_str)
        {
            order.entry(name.to_string()).or_insert(index);
        }
    }
    Ok(order)
}

fn load_prompt_metadata(config_root: &Path) -> Result<HashMap<String, Vec<Input>>> {
    let metadata_path = config_root.join("rovodev/prompt-metadata.yml");
    if !metadata_path.exists() {
        return Ok(HashMap::new());
    }
    let metadata = load_yaml_file(&metadata_path)?;
    if metadata.is_null() {
        return Ok(HashMap::new());
    }
    let mapping = metadata.as_mapping().ok_or_else(|| {
        format!(
            "{}: top-level document must be a mapping",
            metadata_path.display()
        )
    })?;
    let prompts_value = get(mapping, "prompts")
        .cloned()
        .unwrap_or(Value::Mapping(Mapping::new()));
    let prompts = prompts_value
        .as_mapping()
        .ok_or_else(|| format!("{}: prompts must be a mapping", metadata_path.display()))?;
    let mut normalized = HashMap::new();
    for (name_value, prompt_metadata) in prompts {
        let name = name_value.as_str().unwrap_or_default();
        let context = format!("{}: prompts[{name:?}]", metadata_path.display());
        let skill_name = validate_skill_name(Some(name_value), &context)?;
        let prompt_mapping = prompt_metadata
            .as_mapping()
            .ok_or_else(|| format!("{context}: metadata must be a mapping"))?;
        require_known_keys(prompt_mapping, PROMPT_METADATA_KEYS, &context)?;
        normalized.insert(
            skill_name,
            normalize_inputs(get(prompt_mapping, "inputs"), &context)?,
        );
    }
    Ok(normalized)
}

fn collect_skills(config_root: &Path) -> Result<Vec<Prompt>> {
    let skills_root = config_root.join(".agents/skills");
    if !skills_root.is_dir() {
        return Err(format!(
            "{}: canonical skills directory does not exist",
            skills_root.display()
        ));
    }
    let prompt_metadata = load_prompt_metadata(config_root)?;
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
        let skill_file = entry.path().join("SKILL.md");
        if skill_file.is_file() {
            skill_files.push(skill_file);
        }
    }
    skill_files.sort();

    let mut skills = Vec::new();
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
        let description = validate_description(
            get(&metadata, "description"),
            &skill_file.display().to_string(),
        )?;
        let inputs = prompt_metadata.get(&name).cloned().unwrap_or_default();
        skills.push(Prompt {
            content_file: format!("prompts/{name}/SKILL.md"),
            name,
            description,
            inputs,
        });
    }

    let unknown: Vec<String> = prompt_metadata
        .keys()
        .filter(|name| !seen_names.contains(*name))
        .cloned()
        .collect();
    if !unknown.is_empty() {
        return Err(format!(
            "{}: metadata for unknown skills: {unknown:?}",
            config_root.join("rovodev/prompt-metadata.yml").display()
        ));
    }
    if skills.is_empty() {
        return Err(format!("{}: no skills found", skills_root.display()));
    }
    Ok(skills)
}

pub(crate) fn render_registry(config_root: &Path) -> Result<String> {
    dump_registry(&generate_registry(config_root)?)
}

fn generate_registry(config_root: &Path) -> Result<Vec<Prompt>> {
    let order = existing_prompt_order(config_root)?;
    let mut prompts = collect_skills(config_root)?;
    let default_order = order.len();
    prompts.sort_by(|left, right| {
        let left_key = (*order.get(&left.name).unwrap_or(&default_order), &left.name);
        let right_key = (
            *order.get(&right.name).unwrap_or(&default_order),
            &right.name,
        );
        left_key.cmp(&right_key)
    });
    Ok(prompts)
}

fn input_to_value(input: &Input) -> Value {
    let mut mapping = Mapping::new();
    mapping.insert(string_key("name"), Value::String(input.name.clone()));
    mapping.insert(string_key("label"), Value::String(input.label.clone()));
    mapping.insert(
        string_key("description"),
        Value::String(input.description.clone()),
    );
    mapping.insert(string_key("type"), Value::String(input.input_type.clone()));
    mapping.insert(string_key("required"), Value::Bool(input.required));
    Value::Mapping(mapping)
}

fn prompt_to_value(prompt: &Prompt) -> Value {
    let mut mapping = Mapping::new();
    mapping.insert(string_key("name"), Value::String(prompt.name.clone()));
    mapping.insert(
        string_key("description"),
        Value::String(prompt.description.clone()),
    );
    mapping.insert(
        string_key("content_file"),
        Value::String(prompt.content_file.clone()),
    );
    if !prompt.inputs.is_empty() {
        mapping.insert(
            string_key("inputs"),
            Value::Sequence(prompt.inputs.iter().map(input_to_value).collect()),
        );
    }
    Value::Mapping(mapping)
}

fn dump_registry(prompts: &[Prompt]) -> Result<String> {
    let mut root = Mapping::new();
    root.insert(
        string_key("prompts"),
        Value::Sequence(prompts.iter().map(prompt_to_value).collect()),
    );
    let mut rendered = serde_yaml::to_string(&Value::Mapping(root))
        .map_err(|err| format!("cannot render prompt registry: {err}"))?;
    rendered = rendered.replace("---\n", "");
    Ok(rendered)
}

fn resolve_content_path(
    config_root: &Path,
    content_file: Option<&Value>,
    context: &str,
) -> Result<PathBuf> {
    let content_file = content_file
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{context}: content_file must be a non-empty string"))?;
    if content_file.is_empty() {
        return Err(format!(
            "{context}: content_file must be a non-empty string"
        ));
    }
    let path = Path::new(content_file);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(format!(
            "{context}: content_file must be a relative path inside {CONTENT_FILE_PREFIX}"
        ));
    }
    if !content_file.starts_with(CONTENT_FILE_PREFIX) {
        return Err(format!(
            "{context}: content_file must start with {CONTENT_FILE_PREFIX}"
        ));
    }
    if !content_file.ends_with(CONTENT_FILE_SUFFIX) {
        return Err(format!(
            "{context}: content_file must point to a SKILL.md file"
        ));
    }
    Ok(config_root.join("rovodev").join(content_file))
}

fn validate_prompts_adapter(config_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let prompts_link = config_root.join("rovodev/prompts");
    let skills_root = config_root.join(".agents/skills");

    match fs::symlink_metadata(&prompts_link) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let prompts_resolved = prompts_link.canonicalize();
            let skills_resolved = skills_root.canonicalize();
            match (prompts_resolved, skills_resolved) {
                (Ok(prompts_resolved), Ok(skills_resolved))
                    if prompts_resolved == skills_resolved => {}
                (Ok(prompts_resolved), _) => errors.push(format!(
                    "{}: must resolve to {}; got {}",
                    prompts_link.display(),
                    skills_root.display(),
                    prompts_resolved.display()
                )),
                (Err(err), _) => errors.push(format!(
                    "{}: must resolve to {}; got {err}",
                    prompts_link.display(),
                    skills_root.display()
                )),
            }
        }
        _ => errors.push(format!(
            "{}: must be a symlink to {}",
            prompts_link.display(),
            skills_root.display()
        )),
    }

    let config_path = config_root.join("rovodev/prompts.yml");
    let config = match load_yaml_file(&config_path) {
        Ok(config) => config,
        Err(err) => {
            errors.push(err);
            return errors;
        }
    };
    let Some(config_mapping) = config.as_mapping() else {
        errors.push(format!(
            "{}: top-level document must be a mapping",
            config_path.display()
        ));
        return errors;
    };
    let Some(prompts) = get(config_mapping, "prompts").and_then(Value::as_sequence) else {
        errors.push(format!(
            "{}: prompts must be a non-empty list",
            config_path.display()
        ));
        return errors;
    };
    if prompts.is_empty() {
        errors.push(format!(
            "{}: prompts must be a non-empty list",
            config_path.display()
        ));
        return errors;
    }
    let prompt_metadata = match load_prompt_metadata(config_root) {
        Ok(metadata) => metadata,
        Err(err) => {
            errors.push(err);
            return errors;
        }
    };

    let mut seen_names = HashSet::new();
    for (index, prompt) in prompts.iter().enumerate() {
        let context = format!("{}: prompts[{index}]", config_path.display());
        let Some(prompt_mapping) = prompt.as_mapping() else {
            errors.push(format!("{context}: prompt must be a mapping"));
            continue;
        };
        if let Err(err) = require_keys(prompt_mapping, PROMPT_REQUIRED_KEYS, &context) {
            errors.push(err);
            continue;
        }
        if let Err(err) = require_known_keys(prompt_mapping, PROMPT_KEYS, &context) {
            errors.push(err);
        }
        let name = match validate_skill_name(get(prompt_mapping, "name"), &context) {
            Ok(name) => name,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let description = match validate_description(get(prompt_mapping, "description"), &context) {
            Ok(description) => description,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let registry_inputs = match normalize_inputs(get(prompt_mapping, "inputs"), &context) {
            Ok(inputs) => inputs,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let content_path = match resolve_content_path(
            config_root,
            get(prompt_mapping, "content_file"),
            &context,
        ) {
            Ok(path) => path,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        if !seen_names.insert(name.clone()) {
            errors.push(format!("{context}: duplicate prompt name: {name}"));
            continue;
        }
        let expected_content_file = format!("prompts/{name}/SKILL.md");
        if get(prompt_mapping, "content_file").and_then(Value::as_str)
            != Some(expected_content_file.as_str())
        {
            errors.push(format!(
                "{context}: content_file must be {expected_content_file:?} for prompt {name:?}"
            ));
        }
        if !content_path.is_file() {
            errors.push(format!(
                "{context}: content_file does not exist: {}",
                content_path.display()
            ));
            continue;
        }
        match parse_front_matter(&content_path).and_then(|metadata| {
            require_known_keys(
                &metadata,
                SKILL_FRONT_MATTER_KEYS,
                &content_path.display().to_string(),
            )?;
            let front_matter_name =
                validate_skill_name(get(&metadata, "name"), &content_path.display().to_string())?;
            let front_matter_description = validate_description(
                get(&metadata, "description"),
                &content_path.display().to_string(),
            )?;
            Ok((front_matter_name, front_matter_description))
        }) {
            Ok((front_matter_name, front_matter_description)) => {
                if front_matter_name != name {
                    errors.push(format!(
                        "{}: front matter name {front_matter_name:?} does not match registry name {name:?}",
                        content_path.display()
                    ));
                }
                if front_matter_description != description {
                    errors.push(format!(
                        "{}: front matter description does not match registry description",
                        content_path.display()
                    ));
                }
                let metadata_inputs = prompt_metadata.get(&name).cloned().unwrap_or_default();
                if metadata_inputs != registry_inputs {
                    errors.push(format!(
                        "{}: inputs for {name:?} do not match registry inputs",
                        config_root.join("rovodev/prompt-metadata.yml").display()
                    ));
                }
            }
            Err(err) => errors.push(err),
        }
    }

    let unknown_metadata: Vec<String> = prompt_metadata
        .keys()
        .filter(|name| !seen_names.contains(*name))
        .cloned()
        .collect();
    if !unknown_metadata.is_empty() {
        errors.push(format!(
            "{}: metadata for unknown skills: {unknown_metadata:?}",
            config_root.join("rovodev/prompt-metadata.yml").display()
        ));
    }
    errors
}

fn validate_generated_registry(config_root: &Path) -> Vec<String> {
    let config_path = config_root.join("rovodev/prompts.yml");
    match generate_registry(config_root)
        .and_then(|registry| dump_registry(&registry))
        .and_then(|expected| {
            let actual = fs::read_to_string(&config_path).map_err(|err| err.to_string())?;
            if actual != expected {
                Err(format!(
                    "{}: generated content is not up to date; run cargo run -- generate",
                    config_path.display()
                ))
            } else {
                Ok(())
            }
        }) {
        Ok(()) => Vec::new(),
        Err(err) => vec![err],
    }
}

pub(crate) fn validate_registry(config_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    if let Err(err) = collect_skills(config_root) {
        errors.push(err);
    }
    errors.extend(validate_prompts_adapter(config_root));
    errors.extend(validate_generated_registry(config_root));
    errors
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
    fn normalize_inputs_accepts_valid_input_values() {
        let value = yaml_value(
            r#"
- name: ticket_id
  label: Ticket ID
  description: Jira ticket ID
  type: string
  required: true
"#,
        );

        assert_eq!(
            normalize_inputs(Some(&value), "metadata").expect("valid inputs"),
            vec![Input {
                name: "ticket_id".to_string(),
                label: "Ticket ID".to_string(),
                description: "Jira ticket ID".to_string(),
                input_type: "string".to_string(),
                required: true,
            }]
        );
    }

    #[test]
    fn normalize_inputs_rejects_duplicate_names() {
        let value = yaml_value(
            r#"
- name: ticket_id
  label: Ticket ID
  description: Jira ticket ID
  type: string
  required: true
- name: ticket_id
  label: Duplicate ticket ID
  description: Duplicate Jira ticket ID
  type: string
  required: false
"#,
        );

        assert!(normalize_inputs(Some(&value), "metadata").is_err());
    }

    #[test]
    fn normalize_inputs_rejects_non_lower_snake_case_names() {
        for name in ["TicketId", "_ticket_id", "ticket-id"] {
            let value = yaml_value(&format!(
                r#"
- name: {name}
  label: Ticket ID
  description: Jira ticket ID
  type: string
  required: true
"#
            ));

            assert!(normalize_inputs(Some(&value), "metadata").is_err());
        }
    }

    #[test]
    fn normalize_inputs_requires_supported_string_type() {
        let value = yaml_value(
            r#"
- name: ticket_id
  label: Ticket ID
  description: Jira ticket ID
  type: number
  required: true
"#,
        );

        assert!(normalize_inputs(Some(&value), "metadata").is_err());
    }

    #[test]
    fn skill_front_matter_allows_relay_command_registration_key() {
        let value = yaml_value(
            r#"
name: apply-changes
description: Apply a narrow requested code change.
register_cmd: true
"#,
        );
        let metadata = value.as_mapping().expect("front matter mapping");

        require_known_keys(metadata, SKILL_FRONT_MATTER_KEYS, "skill")
            .expect("register_cmd is supported skill metadata");
    }

    #[test]
    fn resolve_content_path_accepts_prompt_skill_paths_only() {
        let config_root = Path::new("/config");
        let valid = Value::String("prompts/apply-changes/SKILL.md".to_string());
        assert_eq!(
            resolve_content_path(config_root, Some(&valid), "prompt").expect("valid content path"),
            PathBuf::from("/config/rovodev/prompts/apply-changes/SKILL.md")
        );

        let absolute = Value::String("/tmp/prompts/apply-changes/SKILL.md".to_string());
        assert!(resolve_content_path(config_root, Some(&absolute), "prompt").is_err());

        let traversal = Value::String("prompts/../secret/SKILL.md".to_string());
        assert!(resolve_content_path(config_root, Some(&traversal), "prompt").is_err());

        let wrong_suffix = Value::String("prompts/apply-changes/README.md".to_string());
        assert!(resolve_content_path(config_root, Some(&wrong_suffix), "prompt").is_err());
    }
}
