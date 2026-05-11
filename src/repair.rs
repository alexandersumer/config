use crate::cli::parse_config_args;
use crate::error::Result;
use crate::links::{link_path, require_dir};

pub(crate) fn repair_config_command(args: &[String]) -> Result<()> {
    let (config_root, _) = parse_config_args(args, false)?;
    let skills_dir = config_root.join(".agents/skills");
    let prompts_link = config_root.join("rovodev/prompts");
    require_dir(&skills_dir, "config skills directory")?;
    link_path(
        &skills_dir,
        &prompts_link,
        "config prompt adapter",
        &config_root,
        &skills_dir,
    )?;
    Ok(())
}
