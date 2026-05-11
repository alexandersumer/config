use crate::commands::{generate_command, validate_command};
use crate::config_root::config_root_from_exe;
use crate::error::Result;
use crate::install::install_command;
use crate::registry::{get, load_yaml_file, string_key, validate_registry};
use crate::repair::repair_config_command;
use serde_yaml::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub(crate) fn run_regression_tests() -> Result<()> {
    assert_clean_fixture_passes()?;
    let mutations: &[(&str, fn(&Path) -> Result<&'static str>)] = &[
        ("missing skill file", mutate_missing_skill_file),
        (
            "front matter name mismatch",
            mutate_front_matter_name_mismatch,
        ),
        ("generated registry drift", mutate_generated_registry_drift),
        ("unregistered skill file", mutate_unregistered_skill_file),
        ("broken prompt adapter", mutate_prompt_adapter_broken),
    ];
    for (name, mutate) in mutations {
        assert_mutation_fails(name, *mutate)?;
    }
    test_repair_config_command()?;
    test_install_command()?;
    test_link_safety()?;
    test_command_failures()?;
    Ok(())
}

fn test_repair_config_command() -> Result<()> {
    // Test the public behavior of repair-config: empty rovodev/prompts directory
    // should be replaced with a relative symlink to ../.agents/skills
    let fixture = copy_fixture()?;
    let prompts_link = fixture.path().join("rovodev/prompts");

    // Remove the existing symlink and create an empty directory instead
    fs::remove_file(&prompts_link).map_err(|err| {
        format!(
            "{}: cannot remove fixture symlink: {err}",
            prompts_link.display()
        )
    })?;
    fs::create_dir(&prompts_link).map_err(|err| {
        format!(
            "{}: cannot create empty directory: {err}",
            prompts_link.display()
        )
    })?;

    // Verify initial state: directory exists
    if !prompts_link.is_dir() {
        return Err("Setup failed: rovodev/prompts should be a directory".to_string());
    }

    // Run repair-config command
    repair_config_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
    ])?;

    // Verify the symlink now exists and points to the correct relative target
    if !prompts_link.is_symlink() {
        return Err(format!(
            "{}: should be a symlink after repair",
            prompts_link.display()
        ));
    }

    // Read the symlink and verify it's a relative path
    let link_target = fs::read_link(&prompts_link)
        .map_err(|err| format!("{}: cannot read symlink: {err}", prompts_link.display()))?;

    if link_target != PathBuf::from("../.agents/skills") {
        return Err(format!(
            "{}: symlink should point to ../.agents/skills; got {:?}",
            prompts_link.display(),
            link_target
        ));
    }

    // Verify the symlink resolves to the skills directory
    let resolved = prompts_link.canonicalize().map_err(|err| {
        format!(
            "{}: cannot canonicalize symlink: {err}",
            prompts_link.display()
        )
    })?;
    let expected = fixture
        .path()
        .join(".agents/skills")
        .canonicalize()
        .map_err(|err| {
            format!(
                "{}: cannot canonicalize skills directory: {err}",
                fixture.path().join(".agents/skills").display()
            )
        })?;

    if resolved != expected {
        return Err(format!(
            "{}: symlink resolves to {}; expected {}",
            prompts_link.display(),
            resolved.display(),
            expected.display()
        ));
    }

    Ok(())
}

pub(crate) fn test_install_command() -> Result<()> {
    let fixture = copy_fixture()?;
    let home = tempfile::Builder::new()
        .prefix("tmp_rovodev_install_home_")
        .tempdir()
        .map_err(|err| format!("cannot create temp install home: {err}"))?;

    install_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])?;

    assert_symlink_resolves_to(
        &home.path().join(".agents"),
        &fixture.path().join(".agents"),
    )?;
    assert_symlink_resolves_to(
        &home.path().join(".rovodev/skills"),
        &fixture.path().join(".agents/skills"),
    )?;
    assert_symlink_resolves_to(
        &home.path().join(".rovodev/prompts"),
        &fixture.path().join(".agents/skills"),
    )?;
    assert_symlink_resolves_to(
        &home.path().join(".rovodev/prompts.yml"),
        &fixture.path().join("rovodev/prompts.yml"),
    )?;
    Ok(())
}

pub(crate) fn test_link_safety() -> Result<()> {
    let repair_fixture = copy_fixture()?;
    let prompts_link = repair_fixture.path().join("rovodev/prompts");
    fs::remove_file(&prompts_link).map_err(|err| {
        format!(
            "{}: cannot remove fixture symlink: {err}",
            prompts_link.display()
        )
    })?;
    fs::create_dir(&prompts_link).map_err(|err| {
        format!(
            "{}: cannot create fixture directory: {err}",
            prompts_link.display()
        )
    })?;
    fs::write(prompts_link.join("keep"), "do not delete").map_err(|err| {
        format!(
            "{}: cannot write fixture file: {err}",
            prompts_link.display()
        )
    })?;
    let repair_error = repair_config_command(&[
        "--config-root".to_string(),
        repair_fixture.path().display().to_string(),
    ])
    .expect_err("repair-config should reject non-empty config prompt adapter directories");
    if !repair_error.contains("not an empty directory") {
        return Err(format!("unexpected repair-config error: {repair_error}"));
    }
    if !prompts_link.join("keep").is_file() {
        return Err(
            "repair-config removed data from non-empty config prompt adapter directory".to_string(),
        );
    }

    let install_fixture = copy_fixture()?;
    let home = tempfile::Builder::new()
        .prefix("tmp_rovodev_install_safety_")
        .tempdir()
        .map_err(|err| format!("cannot create temp install home: {err}"))?;
    fs::create_dir(home.path().join(".agents"))
        .map_err(|err| format!("cannot create fixture .agents directory: {err}"))?;
    fs::write(home.path().join(".agents/keep"), "do not delete")
        .map_err(|err| format!("cannot write fixture .agents file: {err}"))?;
    let install_error = install_command(&[
        "--config-root".to_string(),
        install_fixture.path().display().to_string(),
        "--home".to_string(),
        home.path().display().to_string(),
    ])
    .expect_err("install should reject non-empty home .agents directories");
    if !install_error.contains("not an empty directory") {
        return Err(format!("unexpected install error: {install_error}"));
    }
    if !home.path().join(".agents/keep").is_file() {
        return Err("install removed data from non-empty home .agents directory".to_string());
    }
    Ok(())
}

pub(crate) fn test_command_failures() -> Result<()> {
    let drift_fixture = copy_fixture()?;
    let mut config = load_config(drift_fixture.path())?;
    let prompts = config
        .as_mapping_mut()
        .and_then(|mapping| mapping.get_mut(string_key("prompts")))
        .and_then(Value::as_sequence_mut)
        .ok_or_else(|| "fixture prompts.yml is missing prompts".to_string())?;
    let first_prompt = prompts
        .first_mut()
        .and_then(Value::as_mapping_mut)
        .ok_or_else(|| "fixture prompts.yml is missing prompt mapping".to_string())?;
    first_prompt.insert(
        string_key("description"),
        Value::String("drifted".to_string()),
    );
    write_config(drift_fixture.path(), &config)?;
    let generate_error = generate_command(&[
        "--config-root".to_string(),
        drift_fixture.path().display().to_string(),
        "--check".to_string(),
    ])
    .expect_err("generate --check should fail when prompts.yml has drifted");
    if !generate_error.contains("generated content is not up to date") {
        return Err(format!(
            "unexpected generate --check error: {generate_error}"
        ));
    }

    let invalid_fixture = copy_fixture()?;
    mutate_missing_skill_file(invalid_fixture.path())?;
    let validate_error = validate_command(&[
        "--config-root".to_string(),
        invalid_fixture.path().display().to_string(),
    ])
    .expect_err("validate should fail when validation errors exist");
    if !validate_error.contains("Skill validation failed") {
        return Err(format!("unexpected validate error: {validate_error}"));
    }
    Ok(())
}

fn copy_fixture() -> Result<TempDir> {
    let config_root = config_root_from_exe()?;
    let temp_dir = tempfile::Builder::new()
        .prefix("tmp_rovodev_skill_validation_")
        .tempdir()
        .map_err(|err| format!("cannot create temp fixture: {err}"))?;
    copy_tree(
        &config_root.join(".agents"),
        &temp_dir.path().join(".agents"),
    )?;
    copy_tree(
        &config_root.join("rovodev"),
        &temp_dir.path().join("rovodev"),
    )?;
    Ok(temp_dir)
}

fn copy_tree(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)
        .map_err(|err| format!("{}: cannot create directory: {err}", target.display()))?;
    for entry in walkdir::WalkDir::new(source).follow_links(false) {
        let entry =
            entry.map_err(|err| format!("{}: cannot walk directory: {err}", source.display()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(source)
            .map_err(|err| format!("{}: cannot compute relative path: {err}", path.display()))?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let destination = target.join(relative);
        let file_type = entry.file_type();
        if file_type.is_dir() {
            fs::create_dir_all(&destination).map_err(|err| {
                format!("{}: cannot create directory: {err}", destination.display())
            })?;
        } else if file_type.is_symlink() {
            let link_target = fs::read_link(path)
                .map_err(|err| format!("{}: cannot read symlink: {err}", path.display()))?;
            #[cfg(unix)]
            unix_fs::symlink(&link_target, &destination).map_err(|err| {
                format!("{}: cannot create symlink: {err}", destination.display())
            })?;
            #[cfg(not(unix))]
            return Err(format!("{}: symlink fixtures require unix", path.display()));
        } else if file_type.is_file() {
            fs::copy(path, &destination)
                .map_err(|err| format!("{}: cannot copy file: {err}", path.display()))?;
        }
    }
    Ok(())
}

pub(crate) fn assert_clean_fixture_passes() -> Result<()> {
    let fixture = copy_fixture()?;
    generate_command(&[
        "--config-root".to_string(),
        fixture.path().display().to_string(),
    ])?;
    let errors = validate_registry(fixture.path());
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "clean fixture validation failed:\n{}",
            errors.join("\n")
        ))
    }
}

fn assert_symlink_resolves_to(link: &Path, expected: &Path) -> Result<()> {
    if !link.is_symlink() {
        return Err(format!("{}: expected a symlink", link.display()));
    }
    let actual = link
        .canonicalize()
        .map_err(|err| format!("{}: cannot resolve symlink: {err}", link.display()))?;
    let expected = expected.canonicalize().map_err(|err| {
        format!(
            "{}: cannot resolve expected target: {err}",
            expected.display()
        )
    })?;
    if actual != expected {
        return Err(format!(
            "{}: resolves to {}; expected {}",
            link.display(),
            actual.display(),
            expected.display()
        ));
    }
    Ok(())
}

fn assert_mutation_fails(name: &str, mutate: fn(&Path) -> Result<&'static str>) -> Result<()> {
    let fixture = copy_fixture()?;
    let expected = mutate(fixture.path())?;
    let errors = validate_registry(fixture.path());
    let output = errors.join("\n");
    if errors.is_empty() {
        return Err(format!("{name}: validator unexpectedly passed\n{output}"));
    }
    if !output.contains(expected) {
        return Err(format!("{name}: expected {expected:?} in output\n{output}"));
    }
    Ok(())
}

fn load_config(config_root: &Path) -> Result<Value> {
    load_yaml_file(&config_root.join("rovodev/prompts.yml"))
}

fn write_config(config_root: &Path, config: &Value) -> Result<()> {
    let text = serde_yaml::to_string(config)
        .map_err(|err| format!("cannot render fixture config: {err}"))?;
    fs::write(
        config_root.join("rovodev/prompts.yml"),
        text.replace("---\n", ""),
    )
    .map_err(|err| format!("cannot write fixture config: {err}"))
}

fn first_prompt_name(config_root: &Path) -> Result<String> {
    let config = load_config(config_root)?;
    config
        .as_mapping()
        .and_then(|mapping| get(mapping, "prompts"))
        .and_then(Value::as_sequence)
        .and_then(|prompts| prompts.first())
        .and_then(Value::as_mapping)
        .and_then(|prompt| get(prompt, "name"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| "fixture prompts.yml is missing prompts[0].name".to_string())
}

fn mutate_missing_skill_file(config_root: &Path) -> Result<&'static str> {
    let name = first_prompt_name(config_root)?;
    fs::remove_file(
        config_root
            .join(".agents/skills")
            .join(name)
            .join("SKILL.md"),
    )
    .map_err(|err| format!("cannot remove fixture skill file: {err}"))?;
    Ok("content_file does not exist")
}

fn mutate_front_matter_name_mismatch(config_root: &Path) -> Result<&'static str> {
    let name = first_prompt_name(config_root)?;
    let skill_path = config_root
        .join(".agents/skills")
        .join(&name)
        .join("SKILL.md");
    let text = fs::read_to_string(&skill_path)
        .map_err(|err| format!("{}: cannot read fixture skill: {err}", skill_path.display()))?;
    fs::write(
        &skill_path,
        text.replacen(&format!("name: {name}"), "name: broken-name", 1),
    )
    .map_err(|err| {
        format!(
            "{}: cannot write fixture skill: {err}",
            skill_path.display()
        )
    })?;
    Ok("does not match directory")
}

fn mutate_generated_registry_drift(config_root: &Path) -> Result<&'static str> {
    let mut config = load_config(config_root)?;
    let prompts = config
        .as_mapping_mut()
        .and_then(|mapping| mapping.get_mut(string_key("prompts")))
        .and_then(Value::as_sequence_mut)
        .ok_or_else(|| "fixture prompts.yml is missing prompts".to_string())?;
    let prompt = prompts
        .iter_mut()
        .find(|prompt| {
            prompt
                .as_mapping()
                .and_then(|mapping| get(mapping, "name"))
                .and_then(Value::as_str)
                == Some("apply-changes")
        })
        .and_then(Value::as_mapping_mut)
        .ok_or_else(|| "fixture prompts.yml is missing apply-changes".to_string())?;
    let inputs = prompt
        .get_mut(string_key("inputs"))
        .and_then(Value::as_sequence_mut)
        .ok_or_else(|| "fixture apply-changes prompt is missing inputs".to_string())?;
    let first_input = inputs
        .first_mut()
        .and_then(Value::as_mapping_mut)
        .ok_or_else(|| "fixture apply-changes prompt is missing input mapping".to_string())?;
    first_input.insert(string_key("required"), Value::Bool(true));
    write_config(config_root, &config)?;
    Ok("generated content is not up to date")
}

fn mutate_unregistered_skill_file(config_root: &Path) -> Result<&'static str> {
    let extra_dir = config_root.join(".agents/skills/unregistered-skill");
    fs::create_dir_all(&extra_dir).map_err(|err| {
        format!(
            "{}: cannot create fixture skill: {err}",
            extra_dir.display()
        )
    })?;
    fs::write(
        extra_dir.join("SKILL.md"),
        "---\nname: unregistered-skill\ndescription: Unregistered skill\n---\n\nThis skill is intentionally not listed in the generated registry.\n",
    )
    .map_err(|err| format!("cannot write fixture skill: {err}"))?;
    Ok("generated content is not up to date")
}

fn mutate_prompt_adapter_broken(config_root: &Path) -> Result<&'static str> {
    let prompts_link = config_root.join("rovodev/prompts");
    fs::remove_file(&prompts_link).map_err(|err| {
        format!(
            "{}: cannot remove fixture symlink: {err}",
            prompts_link.display()
        )
    })?;
    #[cfg(unix)]
    unix_fs::symlink("../.agents/missing", &prompts_link).map_err(|err| {
        format!(
            "{}: cannot create fixture symlink: {err}",
            prompts_link.display()
        )
    })?;
    #[cfg(not(unix))]
    return Err("broken prompt adapter fixture requires unix symlinks".to_string());
    Ok("must resolve to")
}
