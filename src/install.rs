use crate::config_root::discover_config_root;
use crate::error::Result;
use crate::links::{link_path, require_dir};
use std::env;
use std::fs;
use std::path::PathBuf;

pub(crate) fn install_command(args: &[String]) -> Result<()> {
    let (config_root, home_dir) = parse_install_args(args)?;
    let agents_dir = config_root.join(".agents");
    let skills_dir = agents_dir.join("skills");
    let rovodev_dir = config_root.join("rovodev");
    let global_agents_dir = home_dir.join(".agents");
    let global_dir = home_dir.join(".rovodev");

    require_dir(&agents_dir, "config agents directory")?;
    require_dir(&skills_dir, "config skills directory")?;
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
