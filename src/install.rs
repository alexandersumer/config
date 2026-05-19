use crate::config_root::discover_config_root;
use crate::error::Result;
use crate::links::{link_path, require_dir};
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

pub(crate) fn install_command(args: &[String]) -> Result<()> {
    let (config_root, home_dir) = parse_install_args(args)?;
    let agents_dir = config_root.join(".agents");
    let skills_dir = agents_dir.join("skills");
    let rovodev_dir = config_root.join("rovodev");
    let global_agents_dir = home_dir.join(".agents");
    let global_dir = home_dir.join(".rovodev");

    require_dir(&agents_dir, "config agents directory")?;
    require_dir(&skills_dir, "config skills directory")?;
    let codex_skill_links = prepare_codex_skill_links(&skills_dir, &home_dir)?;
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
        &config_root,
        &skills_dir,
    )?;
    link_path(
        &skills_dir,
        &global_dir.join("skills"),
        "skills",
        &config_root,
        &skills_dir,
    )?;
    link_path(
        &rovodev_dir.join("prompts.yml"),
        &global_dir.join("prompts.yml"),
        "prompts.yml",
        &config_root,
        &skills_dir,
    )?;
    link_path(
        &skills_dir,
        &global_dir.join("prompts"),
        "prompt adapter",
        &config_root,
        &skills_dir,
    )?;
    apply_codex_skill_links(codex_skill_links)?;

    println!();
    println!(
        "Done. Agent config is now managed from {}.",
        config_root.display()
    );
    println!(
        "~/.agents points at {}, and Rovo Dev skills point at {}.",
        agents_dir.display(),
        skills_dir.display()
    );
    println!("Run /skills to see native skills, or /prompts to use legacy prompt commands.");
    Ok(())
}

fn prepare_codex_skill_links(
    skills_dir: &Path,
    home_dir: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    let codex_skills_dir = home_dir.join(".codex/skills");
    let system_skills_dir = codex_skills_dir.join(".system");
    require_dir(&system_skills_dir, "Codex system skills directory")?;
    verify_codex_system_skills(&system_skills_dir)?;

    let links = codex_skill_links(skills_dir, &codex_skills_dir)?;
    for (source, target) in &links {
        verify_codex_skill_target(source, target)?;
    }
    Ok(links)
}

fn apply_codex_skill_links(links: Vec<(PathBuf, PathBuf)>) -> Result<()> {
    for (source, target) in links {
        link_codex_skill(&source, &target)?;
    }
    Ok(())
}

fn codex_skill_links(
    skills_dir: &Path,
    codex_skills_dir: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    discover_custom_skills(skills_dir)?
        .into_iter()
        .map(|skill_dir| {
            let name = skill_dir
                .file_name()
                .ok_or_else(|| format!("{}: skill directory has no name", skill_dir.display()))?;
            Ok((skill_dir.clone(), codex_skills_dir.join(name)))
        })
        .collect()
}

fn discover_custom_skills(skills_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut skill_dirs = Vec::new();
    for entry in fs::read_dir(skills_dir).map_err(|err| {
        format!(
            "{}: cannot read skills directory: {err}",
            skills_dir.display()
        )
    })? {
        let entry = entry.map_err(|err| {
            format!(
                "{}: cannot inspect skills directory entry: {err}",
                skills_dir.display()
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
        if path.join("SKILL.md").is_file() {
            skill_dirs.push(path);
        }
    }
    skill_dirs.sort();
    Ok(skill_dirs)
}

fn verify_codex_system_skills(system_skills_dir: &Path) -> Result<()> {
    for skill in ["skill-creator", "skill-installer", "openai-docs"] {
        let skill_file = system_skills_dir.join(skill).join("SKILL.md");
        if !skill_file.is_file() {
            return Err(format!(
                "Error: required Codex system skill is missing: {}",
                skill_file.display()
            ));
        }
    }
    Ok(())
}

fn verify_codex_skill_target(source: &Path, target: &Path) -> Result<()> {
    let source_resolved = source.canonicalize().map_err(|err| {
        format!(
            "{}: cannot resolve Codex skill source: {err}",
            source.display()
        )
    })?;

    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let target_resolved = target.canonicalize().map_err(|err| {
                format!(
                    "{}: cannot resolve Codex skill symlink: {err}",
                    target.display()
                )
            })?;
            if target_resolved == source_resolved {
                Ok(())
            } else {
                let existing = fs::read_link(target).map_err(|err| {
                    format!("{}: cannot read Codex skill symlink: {err}", target.display())
                })?;
                Err(format!(
                    "Error: {} is already a symlink to {}\nBack it up and remove it first if you want to replace it.",
                    target.display(),
                    existing.display()
                ))
            }
        }
        Ok(_) => Err(format!(
            "Error: {} already exists and is not the expected Codex skill symlink.\nBack it up and remove it first, then re-run this command.",
            target.display()
        )),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "{}: cannot inspect Codex skill target: {err}",
            target.display()
        )),
    }
}

fn link_codex_skill(source: &Path, target: &Path) -> Result<()> {
    if target.exists() || target.is_symlink() {
        println!("Codex skill already linked correctly: {}", target.display());
    } else {
        create_codex_skill_symlink(source, target)?;
        println!("Linked Codex skill -> {}", source.display());
    }
    Ok(())
}

fn create_codex_skill_symlink(source: &Path, target: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        unix_fs::symlink(source, target).map_err(|err| {
            format!(
                "{}: cannot create Codex skill symlink: {err}",
                target.display()
            )
        })
    }
    #[cfg(not(unix))]
    {
        let _ = (source, target);
        Err("install requires unix symlink support".to_string())
    }
}

fn parse_install_args(args: &[String]) -> Result<(PathBuf, PathBuf)> {
    let mut config_root: Option<PathBuf> = None;
    let mut home_dir: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--config-root" => {
                index += 1;
                config_root = Some(PathBuf::from(
                    args.get(index).ok_or("--config-root requires a path")?,
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

    let config_root = match config_root {
        Some(path) => path,
        None => discover_config_root(
            &env::current_exe()
                .map_err(|err| format!("cannot determine current executable: {err}"))?,
        )?,
    }
    .canonicalize()
    .map_err(|err| format!("cannot resolve config root: {err}"))?;

    let home_dir = match home_dir {
        Some(path) => path,
        None => PathBuf::from(env::var("HOME").map_err(|_| "HOME is not set".to_string())?),
    };

    Ok((config_root, home_dir))
}
