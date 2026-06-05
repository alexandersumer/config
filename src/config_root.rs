use crate::error::Result;
use std::env;
use std::path::{Path, PathBuf};

pub(crate) fn discover_config_root(start: &Path) -> Result<PathBuf> {
    for candidate in start.ancestors() {
        if is_config_root(candidate) {
            return Ok(candidate.to_path_buf());
        }
    }
    Err(format!(
        "cannot determine config root from {}",
        start.display()
    ))
}

pub(crate) fn default_config_root() -> Result<PathBuf> {
    let exe =
        env::current_exe().map_err(|err| format!("cannot determine current executable: {err}"))?;
    match discover_config_root(&exe) {
        Ok(root) => Ok(root),
        Err(exe_error) => match config_root_from_home_agents()? {
            Some(root) => Ok(root),
            None => Err(exe_error),
        },
    }
}

pub(crate) fn config_root_from_exe() -> Result<PathBuf> {
    default_config_root()
}

fn config_root_from_home_agents() -> Result<Option<PathBuf>> {
    let Ok(home) = env::var("HOME") else {
        return Ok(None);
    };
    discover_config_root_from_home(Path::new(&home))
}

fn discover_config_root_from_home(home: &Path) -> Result<Option<PathBuf>> {
    let agents_link = home.join(".agents");
    let agents_dir = match agents_link.canonicalize() {
        Ok(path) => path,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(format!(
                "{}: cannot resolve managed agents link: {err}",
                agents_link.display()
            ));
        }
    };

    let Some(root) = agents_dir.parent() else {
        return Ok(None);
    };
    if is_config_root(root) {
        Ok(Some(root.to_path_buf()))
    } else {
        Ok(None)
    }
}

fn is_config_root(candidate: &Path) -> bool {
    candidate.join(".agents/skills").is_dir() && candidate.join("Cargo.toml").is_file()
}

#[cfg(test)]
mod tests {
    use super::{discover_config_root, discover_config_root_from_home};
    use std::fs;
    use std::path::Path;

    #[test]
    fn discover_config_root_walks_up_from_nested_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("config");
        fs::create_dir_all(root.join(".agents/skills")).expect("agents skills");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"config-tools\"\n",
        )
        .expect("cargo");
        let nested = root.join("src/bin/config-tools");
        fs::create_dir_all(nested.parent().expect("nested parent")).expect("nested dir");
        fs::write(&nested, "binary").expect("nested file");

        assert_eq!(discover_config_root(&nested).expect("root"), root);
    }

    #[cfg(unix)]
    #[test]
    fn discover_config_root_falls_back_to_home_agents_link() {
        use std::os::unix::fs as unix_fs;

        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("config");
        let home = temp.path().join("home");
        fs::create_dir_all(root.join(".agents/skills")).expect("agents skills");
        fs::create_dir_all(&home).expect("home");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"config-tools\"\n",
        )
        .expect("cargo");
        unix_fs::symlink(root.join(".agents"), home.join(".agents")).expect("agents link");

        assert_eq!(
            discover_config_root_from_home(&home).expect("fallback"),
            Some(root.canonicalize().expect("canonical root"))
        );
    }

    #[test]
    fn discover_config_root_returns_none_without_home_agents_link() {
        let temp = tempfile::tempdir().expect("tempdir");
        assert_eq!(
            discover_config_root_from_home(Path::new(temp.path())).expect("fallback"),
            None
        );
    }
}
