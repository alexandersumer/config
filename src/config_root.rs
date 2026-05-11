use crate::error::Result;
use std::env;
use std::path::{Path, PathBuf};

pub(crate) fn discover_config_root(start: &Path) -> Result<PathBuf> {
    for candidate in start.ancestors() {
        if candidate.join(".agents/skills").is_dir() && candidate.join("rovodev").is_dir() {
            return Ok(candidate.to_path_buf());
        }
    }
    Err(format!(
        "cannot determine config root from {}",
        start.display()
    ))
}

pub(crate) fn config_root_from_exe() -> Result<PathBuf> {
    discover_config_root(
        &env::current_exe().map_err(|err| format!("cannot determine current executable: {err}"))?,
    )
}
