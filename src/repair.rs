use crate::cli::parse_repo_args;
use crate::error::Result;
use crate::links::{link_path, require_dir};

pub(crate) fn repair_repo_command(args: &[String]) -> Result<()> {
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
