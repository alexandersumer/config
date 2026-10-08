//! Atomic worker progress records. Git and hook output is diagnostic only.
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Default)]
pub(super) struct Status {
    pub stage: String,
    pub state: Option<String>,
    pub target: Option<String>,
    pub backups: Vec<String>,
    pub complete: bool,
    pub error: Option<String>,
}
impl Status {
    pub fn write(&self, path: &Path) -> Result<(), String> {
        let mut file = tempfile::NamedTempFile::new_in(
            path.parent()
                .ok_or("Worker status requires a parent directory")?,
        )
        .map_err(|e| format!("Cannot create worker status: {e}"))?;
        let value = json!({"stage": self.stage, "state": self.state, "target": self.target,
            "backups": self.backups, "complete": self.complete, "error": self.error});
        file.write_all(value.to_string().as_bytes())
            .map_err(|e| format!("Cannot write worker status: {e}"))?;
        file.persist(path)
            .map_err(|e| format!("Cannot publish worker status: {e}"))?;
        Ok(())
    }
    pub fn read(path: &Path) -> Result<Self, String> {
        let invalid = || "Missing or invalid worker status; inspect its log".to_string();
        let value: Value = serde_json::from_slice(&fs::read(path).map_err(|_| invalid())?)
            .map_err(|_| invalid())?;
        let optional = |key: &str| -> Result<Option<String>, String> {
            match value.get(key) {
                Some(Value::Null) => Ok(None),
                Some(Value::String(s)) => Ok(Some(s.clone())),
                _ => Err(invalid()),
            }
        };
        Ok(Self {
            stage: value["stage"].as_str().ok_or_else(invalid)?.into(),
            state: optional("state")?,
            target: optional("target")?,
            backups: value["backups"]
                .as_array()
                .ok_or_else(invalid)?
                .iter()
                .map(|v| v.as_str().map(str::to_owned).ok_or_else(invalid))
                .collect::<Result<_, _>>()?,
            complete: value["complete"].as_bool().ok_or_else(invalid)?,
            error: optional("error")?,
        })
    }
}
