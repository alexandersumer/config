//! Private versioned intent and outcome records. Never persist command/provider transcripts.
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::{
    ffi::OsStringExt,
    fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
};
use std::path::{Path, PathBuf};

fn root() -> Result<PathBuf, String> {
    let root = std::env::var_os("WORKCTL_STATE_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/state/workctl")))
        .ok_or("HOME unavailable; cannot persist operation intent")?;
    if !root.exists() {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&root)
            .map_err(|e| format!("history directory: {e}"))?;
    }
    if let Some(parent) = root.parent() {
        fs::File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|e| format!("history parent sync failed: {e}"))?;
    }
    let m = fs::symlink_metadata(&root).map_err(|e| e.to_string())?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.permissions().mode() & 0o077 != 0 {
        return Err(
            "history directory must be an owned private directory (0700), not a symlink".into(),
        );
    }
    Ok(root)
}
use std::os::unix::fs::DirBuilderExt;
pub(crate) fn target(path: &Path, common: &Path) -> Value {
    target_with_identity(
        path,
        common,
        fs::symlink_metadata(path).ok().map(|m| (m.dev(), m.ino())),
    )
}
pub(crate) fn target_with_identity(
    path: &Path,
    common: &Path,
    identity: Option<(u64, u64)>,
) -> Value {
    let common_metadata = fs::symlink_metadata(common).ok();
    json!({"path":path.to_string_lossy(),"path_bytes":crate::presentation::path_bytes(path),"common_bytes":crate::presentation::path_bytes(common),"common_device":common_metadata.as_ref().map(|m|m.dev()),"common_inode":common_metadata.as_ref().map(|m|m.ino()),"device":identity.map(|(device,_)|device),"inode":identity.map(|(_,inode)|inode),"status":"pending","recovery_refs":[]})
}
pub(crate) struct Run {
    directory: PathBuf,
    pub id: String,
    value: Value,
}
impl Run {
    pub fn begin(
        operation: &str,
        selection: Value,
        options: Value,
        targets: Vec<Value>,
    ) -> Result<Self, String> {
        let root = root()?;
        let directory = tempfile::Builder::new()
            .prefix("run-")
            .tempdir_in(&root)
            .map_err(|e| e.to_string())?
            .keep();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
        let id = directory
            .file_name()
            .ok_or("invalid run directory")?
            .to_string_lossy()
            .into_owned();
        let value = json!({"schema_version":1,"run_id":id,"operation":operation,"status":"in_progress","started_unix_seconds":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_secs_f64(),"selection":selection,"options":options,"targets":targets});
        let run = Self {
            directory,
            id,
            value,
        };
        run.publish()?;
        fs::File::open(&root)
            .and_then(|f| f.sync_all())
            .map_err(|e| {
                format!("history intent directory sync failed; no mutation authorized: {e}")
            })?;
        writeln!(std::io::stderr(), "Run: {}", run.id).map_err(|e| e.to_string())?;
        Ok(run)
    }
    fn publish(&self) -> Result<(), String> {
        let mut staged = tempfile::NamedTempFile::new_in(&self.directory)
            .map_err(|e| format!("history write failed; run may be incomplete: {e}"))?;
        staged
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
        staged
            .write_all(&serde_json::to_vec(&self.value).map_err(|e| e.to_string())?)
            .and_then(|_| staged.as_file().sync_all())
            .map_err(|e| {
                format!("history write failed; mutation may already have occurred: {e}")
            })?;
        staged
            .persist(self.directory.join("summary.json"))
            .map_err(|e| {
                format!("history publication failed; mutation may already have occurred: {e}")
            })?;
        fs::File::open(&self.directory)
            .and_then(|f| f.sync_all())
            .map_err(|e| format!("history directory sync failed; evidence incomplete: {e}"))
    }
    pub fn outcome(&mut self, path: &Path, status: &str, refs: Vec<String>) -> Result<(), String> {
        let target = self.value["targets"]
            .as_array_mut()
            .ok_or("invalid history intent")?
            .iter_mut()
            .find(|v| v["path_bytes"] == json!(crate::presentation::path_bytes(path)))
            .ok_or("outcome outside recorded selection")?;
        target["status"] = json!(status);
        target["recovery_refs"] = json!(refs);
        self.publish()
    }
    pub fn evidence(&mut self, path: &Path, evidence: Value) -> Result<(), String> {
        let target = self.value["targets"]
            .as_array_mut()
            .ok_or("invalid history targets")?
            .iter_mut()
            .find(|v| v["path_bytes"] == json!(crate::presentation::path_bytes(path)))
            .ok_or("evidence outside recorded selection")?;
        target["evidence"] = evidence;
        self.publish()
    }
    pub fn finish(&mut self, status: &str) -> Result<(), String> {
        self.value["status"] = json!(status);
        self.publish()
    }
}
fn validate_id(id: &str) -> Result<(), String> {
    if !id.starts_with("run-")
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("invalid run ID".into());
    }
    Ok(())
}
pub(crate) fn read(id: &str) -> Result<Value, String> {
    validate_id(id)?;
    let dir = root()?.join(id);
    let m = fs::symlink_metadata(&dir).map_err(|e| e.to_string())?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
        return Err("run directory is not private and owned".into());
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(dir.join("summary.json"))
        .map_err(|e| e.to_string())?;
    let m = file.metadata().map_err(|e| e.to_string())?;
    if !m.is_file() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
        return Err("run summary is not a private owned regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(4_194_305)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4_194_304 {
        return Err("run record exceeds 4 MiB".into());
    }
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| "malformed or incomplete run summary")?;
    if value["schema_version"] != 1 || value["run_id"] != id || !value["targets"].is_array() {
        return Err("unsupported or invalid run summary".into());
    }
    Ok(value)
}
pub(crate) fn unsuccessful(target: &Value) -> bool {
    !matches!(
        target["status"].as_str(),
        Some("succeeded" | "removed" | "protected" | "would remove" | "reconciled_absent")
    )
}
pub(crate) fn path(target: &Value, field: &str) -> Result<PathBuf, String> {
    let bytes: Vec<u8> = serde_json::from_value(target[field].clone())
        .map_err(|_| "invalid byte-safe history path")?;
    let p = PathBuf::from(std::ffi::OsString::from_vec(bytes));
    if !p.is_absolute()
        || p.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err("history target must be an absolute normalized path".into());
    }
    Ok(p)
}
pub(crate) fn verify_identity(target: &Value) -> Result<PathBuf, String> {
    let path = path(target, "path_bytes")?;
    let m = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !m.is_dir()
        || target["device"].as_u64() != Some(m.dev())
        || target["inode"].as_u64() != Some(m.ino())
    {
        return Err("history target identity changed; refusing path reuse".into());
    }
    let common = path_from_common(target)?;
    let common_metadata = fs::symlink_metadata(common).map_err(|e| e.to_string())?;
    if !common_metadata.is_dir()
        || target["common_device"].as_u64() != Some(common_metadata.dev())
        || target["common_inode"].as_u64() != Some(common_metadata.ino())
    {
        return Err("history repository identity changed; refusing metadata path reuse".into());
    }
    Ok(path)
}
fn path_from_common(target: &Value) -> Result<PathBuf, String> {
    path(target, "common_bytes")
}
pub(crate) fn command(id: Option<String>, only_unsuccessful: bool, json_output: bool) -> u8 {
    let result = (|| -> Result<Value, String> {
        if let Some(id) = id {
            let mut value = read(&id)?;
            if only_unsuccessful {
                value["targets"]
                    .as_array_mut()
                    .ok_or("invalid history targets")?
                    .retain(unsuccessful);
            }
            Ok(value)
        } else {
            let mut runs = Vec::new();
            for e in fs::read_dir(root()?).map_err(|e| e.to_string())? {
                let e = e.map_err(|e| e.to_string())?;
                let id = e.file_name().to_string_lossy().into_owned();
                if !id.starts_with("run-") {
                    continue;
                }
                match read(&id) {
                    Ok(v) => {
                        if !only_unsuccessful
                            || v["status"] != "completed"
                            || v["targets"]
                                .as_array()
                                .is_some_and(|t| t.iter().any(unsuccessful))
                        {
                            runs.push(json!({"run_id":id,"operation":v["operation"],"status":v["status"],"started_unix_seconds":v["started_unix_seconds"]}));
                        }
                    }
                    Err(e) => {
                        runs.push(json!({"run_id":id,"status":"incomplete_record","error":e}))
                    }
                }
            }
            runs.sort_by(|a, b| a["run_id"].as_str().cmp(&b["run_id"].as_str()));
            Ok(json!({"schema_version":1,"operation":"history.list","runs":runs}))
        }
    })();
    match result {
        Ok(value) => {
            if json_output {
                crate::presentation::json(&value)
            } else {
                u8::from(
                    writeln!(
                        std::io::stdout(),
                        "{}",
                        crate::presentation::safe_text(
                            &serde_json::to_string_pretty(&value).unwrap_or_default()
                        )
                    )
                    .is_err(),
                )
            }
        }
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "workctl history: {e}");
            if json_output {
                let _ = crate::presentation::json(
                    &json!({"schema_version":1,"operation":"history","status":"failed","error":e}),
                );
            }
            1
        }
    }
}

pub(crate) fn safe_name(value: &str) -> Option<&str> {
    (!value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-/".contains(&b)))
    .then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intent_keeps_selected_identity_when_a_directory_is_replaced_before_publication() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("selected");
        let common = temp.path().join("common");
        fs::create_dir(&path).unwrap();
        fs::create_dir(&common).unwrap();
        let original = fs::symlink_metadata(&path).unwrap();
        let identity = Some((original.dev(), original.ino()));
        fs::rename(&path, temp.path().join("original")).unwrap();
        fs::create_dir(&path).unwrap();
        let intent = target_with_identity(&path, &common, identity);
        assert!(verify_identity(&intent)
            .unwrap_err()
            .contains("identity changed"));
        let missing_selection = target_with_identity(&path, &common, None);
        assert!(verify_identity(&missing_selection).is_err());
    }
}
