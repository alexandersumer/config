use regex::Regex;
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const INPUT_KEYS: &[&str] = &["name", "label", "description", "type", "required"];
const SKILL_FRONT_MATTER_KEYS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
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

type Result<T> = std::result::Result<T, String>;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("help");
    match command {
        "generate" => {
            args.remove(0);
            generate_command(&args)
        }
        "validate" => {
            args.remove(0);
            validate_command(&args)
        }
        "test-validate" => {
            args.remove(0);
            test_validate_command(&args)
        }
        "check" => {
            args.remove(0);
            check_command(&args)
        }
        "install-git-hooks" => {
            args.remove(0);
            install_git_hooks_command(&args)
        }
        "install" => {
            args.remove(0);
            install_command(&args)
        }
        "repair-repo" => {
            args.remove(0);
            repair_repo_command(&args)
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command: {other}\n\n{}", help_text())),
    }
}

fn print_help() {
    println!("{}", help_text());
}

fn help_text() -> &'static str {
    "Usage: config-tools <command> [options]\n\nCommands:\n  generate [--repo-root PATH] [--check]\n  validate [--repo-root PATH]\n  test-validate\n  check [--repo-root PATH]\n  install-git-hooks [--repo-root PATH]\n  repair-repo [--repo-root PATH]\n  install [--repo-root PATH] [--home PATH]\n"
}

fn parse_repo_args(args: &[String], allow_check: bool) -> Result<(PathBuf, bool)> {
    let mut repo_root: Option<PathBuf> = None;
    let mut check = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                let value = args.get(index).ok_or("--repo-root requires a path")?;
                repo_root = Some(PathBuf::from(value));
            }
            "--check" if allow_check => check = true,
            "--help" | "-h" => return Err(help_text().to_string()),
            unknown => return Err(format!("unknown option: {unknown}")),
        }
        index += 1;
    }

    let root = match repo_root {
        Some(path) => path,
        None => discover_repo_root(
            &env::current_exe()
                .map_err(|err| format!("cannot determine current executable: {err}"))?,
        )?,
    };
    root.canonicalize()
        .map(|root| (root, check))
        .map_err(|err| format!("{}: cannot resolve repository root: {err}", root.display()))
}

fn generate_command(args: &[String]) -> Result<()> {
    let (repo_root, check) = parse_repo_args(args, true)?;
    let output_path = repo_root.join("rovodev/prompts.yml");
    let rendered = dump_registry(&generate_registry(&repo_root)?)?;

    if check {
        let current = fs::read_to_string(&output_path).map_err(|err| {
            format!(
                "Prompt generation check failed:\n- {}: cannot read file: {err}",
                output_path.display()
            )
        })?;
        if current != rendered {
            return Err(format!(
                "Prompt generation check failed:\n- {}: generated content is not up to date",
                output_path.display()
            ));
        }
        println!("Prompt registry is up to date.");
        return Ok(());
    }

    fs::write(&output_path, rendered)
        .map_err(|err| format!("{}: cannot write file: {err}", output_path.display()))?;
    println!("Generated {}", output_path.display());
    Ok(())
}

fn validate_command(args: &[String]) -> Result<()> {
    let (repo_root, _) = parse_repo_args(args, false)?;
    let errors = validate_registry(&repo_root);
    if !errors.is_empty() {
        let mut output = String::from("Skill validation failed:");
        for error in errors {
            output.push_str("\n- ");
            output.push_str(&error);
        }
        return Err(output);
    }
    println!("Skill validation passed.");
    Ok(())
}

fn test_validate_command(args: &[String]) -> Result<()> {
    if !args.is_empty() {
        return Err(format!("unknown option: {}", args[0]));
    }
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
    test_repair_repo_command()?;
    test_install_command()?;
    test_link_safety()?;
    test_command_failures()?;
    println!("Skill validator regression tests passed.");
    Ok(())
}

fn check_command(args: &[String]) -> Result<()> {
    let (repo_root, _) = parse_repo_args(args, false)?;
    run_cargo(&repo_root, &["fmt", "--check"])?;
    run_cargo(&repo_root, &["check"])?;
    generate_command(&[
        "--repo-root".to_string(),
        repo_root.display().to_string(),
        "--check".to_string(),
    ])?;
    validate_command(&["--repo-root".to_string(), repo_root.display().to_string()])?;
    test_validate_command(&[])?;
    Ok(())
}

fn install_git_hooks_command(args: &[String]) -> Result<()> {
    let (repo_root, _) = parse_repo_args(args, false)?;
    let hooks_dir = repo_root.join(".githooks");
    let hook_path = hooks_dir.join("pre-commit");
    if !hook_path.is_file() {
        return Err(format!(
            "{}: tracked pre-commit hook is missing",
            hook_path.display()
        ));
    }
    run_git(&repo_root, &["config", "core.hooksPath", ".githooks"])?;
    println!(
        "Configured core.hooksPath=.githooks for {}",
        repo_root.display()
    );
    Ok(())
}

fn run_cargo(repo_root: &Path, args: &[&str]) -> Result<()> {
    run_command(repo_root, "cargo", args)
}

fn run_git(repo_root: &Path, args: &[&str]) -> Result<()> {
    run_command(repo_root, "git", args)
}

fn run_command(repo_root: &Path, program: &str, args: &[&str]) -> Result<()> {
    let status = std::process::Command::new(program)
        .args(args)
        .current_dir(repo_root)
        .status()
        .map_err(|err| format!("cannot run {program} {}: {err}", args.join(" ")))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed with {status}", args.join(" ")))
    }
}

fn repair_repo_command(args: &[String]) -> Result<()> {
    let (repo_root, _) = parse_repo_args(args, false)?;
    let skills_dir = repo_root.join(".agents/skills");
    let prompts_link = repo_root.join("rovodev/prompts");
    require_dir(&skills_dir, "repo skills directory")?;
    link_path(
        &skills_dir,
        &prompts_link,
        "repo prompt adapter",
        &repo_root,
        &skills_dir,
    )?;
    Ok(())
}

fn test_repair_repo_command() -> Result<()> {
    // Test the public behavior of repair-repo: empty rovodev/prompts directory
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

    // Run repair-repo command
    repair_repo_command(&[
        "--repo-root".to_string(),
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

fn test_install_command() -> Result<()> {
    let fixture = copy_fixture()?;
    let home = tempfile::Builder::new()
        .prefix("tmp_rovodev_install_home_")
        .tempdir()
        .map_err(|err| format!("cannot create temp install home: {err}"))?;

    install_command(&[
        "--repo-root".to_string(),
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

fn test_link_safety() -> Result<()> {
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
    let repair_error = repair_repo_command(&[
        "--repo-root".to_string(),
        repair_fixture.path().display().to_string(),
    ])
    .expect_err("repair-repo should reject non-empty repo prompt adapter directories");
    if !repair_error.contains("not an empty directory") {
        return Err(format!("unexpected repair-repo error: {repair_error}"));
    }
    if !prompts_link.join("keep").is_file() {
        return Err(
            "repair-repo removed data from non-empty repo prompt adapter directory".to_string(),
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
        "--repo-root".to_string(),
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

fn test_command_failures() -> Result<()> {
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
        "--repo-root".to_string(),
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
        "--repo-root".to_string(),
        invalid_fixture.path().display().to_string(),
    ])
    .expect_err("validate should fail when validation errors exist");
    if !validate_error.contains("Skill validation failed") {
        return Err(format!("unexpected validate error: {validate_error}"));
    }
    Ok(())
}

fn install_command(args: &[String]) -> Result<()> {
    let (repo_root, home_dir) = parse_install_args(args)?;
    let agents_dir = repo_root.join(".agents");
    let skills_dir = agents_dir.join("skills");
    let rovodev_dir = repo_root.join("rovodev");
    let global_agents_dir = home_dir.join(".agents");
    let global_dir = home_dir.join(".rovodev");

    require_dir(&agents_dir, "repo agents directory")?;
    require_dir(&skills_dir, "repo skills directory")?;
    fs::create_dir_all(&global_dir).map_err(|err| {
        format!(
            "{}: cannot create Rovo Dev config directory: {err}",
            global_dir.display()
        )
    })?;

    link_path(
        &agents_dir,
        &global_agents_dir,
        "global agents directory",
        &repo_root,
        &skills_dir,
    )?;
    link_path(
        &skills_dir,
        &global_dir.join("skills"),
        "skills",
        &repo_root,
        &skills_dir,
    )?;
    link_path(
        &rovodev_dir.join("prompts.yml"),
        &global_dir.join("prompts.yml"),
        "prompts.yml",
        &repo_root,
        &skills_dir,
    )?;
    link_path(
        &skills_dir,
        &global_dir.join("prompts"),
        "prompt adapter",
        &repo_root,
        &skills_dir,
    )?;

    println!();
    println!(
        "Done. Agent config is now managed from {}.",
        repo_root.display()
    );
    println!(
        "~/.agents points at {}, and Rovo Dev skills point at {}.",
        agents_dir.display(),
        skills_dir.display()
    );
    println!("Run /skills to see native skills, or /prompts to use legacy prompt commands.");
    Ok(())
}

fn parse_install_args(args: &[String]) -> Result<(PathBuf, PathBuf)> {
    let mut repo_root: Option<PathBuf> = None;
    let mut home_dir: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--repo-root" => {
                index += 1;
                repo_root = Some(PathBuf::from(
                    args.get(index).ok_or("--repo-root requires a path")?,
                ));
            }
            "--home" => {
                index += 1;
                home_dir = Some(PathBuf::from(
                    args.get(index).ok_or("--home requires a path")?,
                ));
            }
            unknown => return Err(format!("unknown option: {unknown}")),
        }
        index += 1;
    }

    let repo_root = match repo_root {
        Some(path) => path,
        None => discover_repo_root(
            &env::current_exe()
                .map_err(|err| format!("cannot determine current executable: {err}"))?,
        )?,
    }
    .canonicalize()
    .map_err(|err| format!("cannot resolve repository root: {err}"))?;

    let home_dir = match home_dir {
        Some(path) => path,
        None => PathBuf::from(env::var("HOME").map_err(|_| "HOME is not set".to_string())?),
    };

    Ok((repo_root, home_dir))
}

fn require_dir(path: &Path, label: &str) -> Result<()> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(format!("Error: {label} is missing: {}", path.display()))
    }
}

fn link_path(
    source: &Path,
    target: &Path,
    label: &str,
    repo_root: &Path,
    skills_dir: &Path,
) -> Result<()> {
    if !source.exists() {
        return Err(format!(
            "Error: source for {label} does not exist: {}",
            source.display()
        ));
    }

    let source_resolved = source
        .canonicalize()
        .map_err(|err| format!("{}: cannot resolve source: {err}", source.display()))?;

    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let existing = fs::read_link(target)
                .map_err(|err| format!("{}: cannot read symlink: {err}", target.display()))?;
            let existing_resolved = resolve_link_target(target, &existing)?;
            let legacy_prompts_resolved =
                normalize_existing_ancestor(&repo_root.join(".agents/prompts"));
            if existing_resolved == normalize_path(&source_resolved) {
                println!("{label} already linked correctly.");
            } else if existing_resolved == legacy_prompts_resolved
                && normalize_path(&source_resolved)
                    == normalize_path(&skills_dir.canonicalize().map_err(|err| {
                        format!(
                            "{}: cannot resolve skills directory: {err}",
                            skills_dir.display()
                        )
                    })?)
            {
                fs::remove_file(target).map_err(|err| {
                    format!("{}: cannot remove legacy symlink: {err}", target.display())
                })?;
                create_symlink(source, target, repo_root)?;
                println!(
                    "Migrated {label} from legacy prompts -> {}",
                    source.display()
                );
            } else {
                return Err(format!(
                    "Error: {} is already a symlink to {}\nRemove it first if you want to replace it.",
                    target.display(),
                    existing.display()
                ));
            }
        }
        Ok(metadata) if metadata.is_dir() => {
            if fs::read_dir(target)
                .map_err(|err| format!("{}: cannot inspect directory: {err}", target.display()))?
                .next()
                .is_none()
            {
                fs::remove_dir(target).map_err(|err| {
                    format!("{}: cannot remove empty directory: {err}", target.display())
                })?;
                create_symlink(source, target, repo_root)?;
                println!(
                    "Replaced empty {label} directory with symlink -> {}",
                    source.display()
                );
            } else {
                return Err(format!(
                    "Error: {} already exists and is not an empty directory.\nBack it up and remove it first, then re-run this command.",
                    target.display()
                ));
            }
        }
        Ok(_) => {
            return Err(format!(
                "Error: {} already exists and is not an empty directory.\nBack it up and remove it first, then re-run this command.",
                target.display()
            ));
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            create_symlink(source, target, repo_root)?;
            println!("Linked {label} -> {}", source.display());
        }
        Err(err) => {
            return Err(format!(
                "{}: cannot inspect target: {err}",
                target.display()
            ))
        }
    }
    Ok(())
}

fn resolve_link_target(link_path: &Path, link_target: &Path) -> Result<PathBuf> {
    let resolved = if link_target.is_absolute() {
        link_target.to_path_buf()
    } else {
        link_path
            .parent()
            .ok_or_else(|| format!("{}: symlink has no parent", link_path.display()))?
            .join(link_target)
    };
    Ok(normalize_existing_ancestor(&resolved))
}

fn normalize_existing_ancestor(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    let mut missing = Vec::new();
    loop {
        if current.exists() {
            if let Ok(mut resolved) = current.canonicalize() {
                for component in missing.iter().rev() {
                    resolved.push(component);
                }
                return normalize_path(&resolved);
            }
        }
        let Some(file_name) = current.file_name().map(|name| name.to_os_string()) else {
            break;
        };
        missing.push(file_name);
        if !current.pop() {
            break;
        }
    }
    normalize_path(path)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn create_symlink(source: &Path, target: &Path, repo_root: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let link_source = symlink_source(source, target, repo_root);
        unix_fs::symlink(&link_source, target)
            .map_err(|err| format!("{}: cannot create symlink: {err}", target.display()))
    }
    #[cfg(not(unix))]
    {
        let _ = (source, target, repo_root);
        Err("install requires unix symlink support".to_string())
    }
}

fn symlink_source(source: &Path, target: &Path, repo_root: &Path) -> PathBuf {
    if source.starts_with(repo_root) && target.starts_with(repo_root) {
        if let Some(target_parent) = target.parent() {
            if let (Ok(source_rel), Ok(parent_rel)) = (
                source.strip_prefix(repo_root),
                target_parent.strip_prefix(repo_root),
            ) {
                let mut relative = PathBuf::new();
                for component in parent_rel.components() {
                    if matches!(component, std::path::Component::Normal(_)) {
                        relative.push("..");
                    }
                }
                relative.push(source_rel);
                return relative;
            }
        }
    }
    source.to_path_buf()
}

fn load_yaml_file(path: &Path) -> Result<Value> {
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
    let re = Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").expect("valid regex");
    if !re.is_match(name) {
        return Err(format!(
            "{context}: skill name must be kebab-case: {name:?}"
        ));
    }
    Ok(name.to_string())
}

fn validate_input_name(value: Option<&Value>, context: &str) -> Result<String> {
    let name = value
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{context}: input name must be a non-empty string"))?;
    if name.is_empty() {
        return Err(format!("{context}: input name must be a non-empty string"));
    }
    let re = Regex::new(r"^[a-z][a-z0-9_]*$").expect("valid regex");
    if !re.is_match(name) {
        return Err(format!(
            "{context}: input name must be lower_snake_case: {name:?}"
        ));
    }
    Ok(name.to_string())
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
    Ok(description.to_string())
}

fn string_key(key: &str) -> Value {
    Value::String(key.to_string())
}

fn get<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
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

fn existing_prompt_order(repo_root: &Path) -> Result<HashMap<String, usize>> {
    let config_path = repo_root.join("rovodev/prompts.yml");
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

fn load_prompt_metadata(repo_root: &Path) -> Result<HashMap<String, Vec<Input>>> {
    let metadata_path = repo_root.join("rovodev/prompt-metadata.yml");
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

fn collect_skills(repo_root: &Path) -> Result<Vec<Prompt>> {
    let skills_root = repo_root.join(".agents/skills");
    if !skills_root.is_dir() {
        return Err(format!(
            "{}: canonical skills directory does not exist",
            skills_root.display()
        ));
    }
    let prompt_metadata = load_prompt_metadata(repo_root)?;
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
        let skill_dir = skill_file
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("{}: invalid skill directory", skill_file.display()))?;
        validate_skill_name(
            Some(&Value::String(skill_dir.to_string())),
            &skill_file.parent().unwrap().display().to_string(),
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
            repo_root.join("rovodev/prompt-metadata.yml").display()
        ));
    }
    if skills.is_empty() {
        return Err(format!("{}: no skills found", skills_root.display()));
    }
    Ok(skills)
}

fn generate_registry(repo_root: &Path) -> Result<Vec<Prompt>> {
    let order = existing_prompt_order(repo_root)?;
    let mut prompts = collect_skills(repo_root)?;
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
    repo_root: &Path,
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
    Ok(repo_root.join("rovodev").join(content_file))
}

fn validate_prompts_adapter(repo_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let prompts_link = repo_root.join("rovodev/prompts");
    let skills_root = repo_root.join(".agents/skills");

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

    let config_path = repo_root.join("rovodev/prompts.yml");
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
    let prompt_metadata = match load_prompt_metadata(repo_root) {
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
        let content_path =
            match resolve_content_path(repo_root, get(prompt_mapping, "content_file"), &context) {
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
                        repo_root.join("rovodev/prompt-metadata.yml").display()
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
            repo_root.join("rovodev/prompt-metadata.yml").display()
        ));
    }
    errors
}

fn validate_generated_registry(repo_root: &Path) -> Vec<String> {
    let config_path = repo_root.join("rovodev/prompts.yml");
    match generate_registry(repo_root)
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

fn validate_registry(repo_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    if let Err(err) = collect_skills(repo_root) {
        errors.push(err);
    }
    errors.extend(validate_prompts_adapter(repo_root));
    errors.extend(validate_generated_registry(repo_root));
    errors
}

fn discover_repo_root(start: &Path) -> Result<PathBuf> {
    for candidate in start.ancestors() {
        if candidate.join(".agents/skills").is_dir() && candidate.join("rovodev").is_dir() {
            return Ok(candidate.to_path_buf());
        }
    }
    Err(format!(
        "cannot determine repository root from {}",
        start.display()
    ))
}

fn repo_root_from_exe() -> Result<PathBuf> {
    discover_repo_root(
        &env::current_exe().map_err(|err| format!("cannot determine current executable: {err}"))?,
    )
}

fn copy_fixture() -> Result<TempDir> {
    let repo_root = repo_root_from_exe()?;
    let temp_dir = tempfile::Builder::new()
        .prefix("tmp_rovodev_skill_validation_")
        .tempdir()
        .map_err(|err| format!("cannot create temp fixture: {err}"))?;
    copy_tree(&repo_root.join(".agents"), &temp_dir.path().join(".agents"))?;
    copy_tree(&repo_root.join("rovodev"), &temp_dir.path().join("rovodev"))?;
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

fn assert_clean_fixture_passes() -> Result<()> {
    let fixture = copy_fixture()?;
    generate_command(&[
        "--repo-root".to_string(),
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

fn load_config(repo_root: &Path) -> Result<Value> {
    load_yaml_file(&repo_root.join("rovodev/prompts.yml"))
}

fn write_config(repo_root: &Path, config: &Value) -> Result<()> {
    let text = serde_yaml::to_string(config)
        .map_err(|err| format!("cannot render fixture config: {err}"))?;
    fs::write(
        repo_root.join("rovodev/prompts.yml"),
        text.replace("---\n", ""),
    )
    .map_err(|err| format!("cannot write fixture config: {err}"))
}

fn first_prompt_name(repo_root: &Path) -> Result<String> {
    let config = load_config(repo_root)?;
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

fn mutate_missing_skill_file(repo_root: &Path) -> Result<&'static str> {
    let name = first_prompt_name(repo_root)?;
    fs::remove_file(repo_root.join(".agents/skills").join(name).join("SKILL.md"))
        .map_err(|err| format!("cannot remove fixture skill file: {err}"))?;
    Ok("content_file does not exist")
}

fn mutate_front_matter_name_mismatch(repo_root: &Path) -> Result<&'static str> {
    let name = first_prompt_name(repo_root)?;
    let skill_path = repo_root
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

fn mutate_generated_registry_drift(repo_root: &Path) -> Result<&'static str> {
    let mut config = load_config(repo_root)?;
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
    write_config(repo_root, &config)?;
    Ok("generated content is not up to date")
}

fn mutate_unregistered_skill_file(repo_root: &Path) -> Result<&'static str> {
    let extra_dir = repo_root.join(".agents/skills/unregistered-skill");
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

fn mutate_prompt_adapter_broken(repo_root: &Path) -> Result<&'static str> {
    let prompts_link = repo_root.join("rovodev/prompts");
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
