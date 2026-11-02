//! Shared, credential-free workspace scope and creation-age policy.
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Default, clap::Args)]
pub(crate) struct ScopeArgs {
    /// Reusable root set; mutually exclusive with positional paths
    #[arg(long, conflicts_with = "paths")]
    pub scope: Option<String>,
    /// JSON workspace policy (default: ~/.config/workctl/workspaces.json)
    #[arg(long, requires = "scope")]
    pub workspace_config: Option<PathBuf>,
    /// Protect this path, descendants, and any containing worktree/repository; repeatable
    #[arg(long, value_name = "PATH", value_hint = clap::ValueHint::AnyPath)]
    pub exclude: Vec<PathBuf>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: u32,
    scopes: BTreeMap<String, Scope>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    roots: Vec<PathBuf>,
    #[serde(default)]
    exclusions: Vec<PathBuf>,
}
pub(crate) struct Resolved {
    pub roots: Vec<PathBuf>,
    pub exclusions: BTreeSet<PathBuf>,
    pub has_symlink_roots: bool,
}
impl Resolved {
    pub fn excluded(&self, path: &Path) -> bool {
        self.exclusions.iter().any(|root| path.starts_with(root))
    }
    pub fn protected_target(&self, path: &Path) -> bool {
        self.excluded(path)
            || self
                .exclusions
                .iter()
                .any(|excluded| excluded.starts_with(path))
    }
    pub fn evidence(&self) -> Value {
        let paths = |paths: Vec<&PathBuf>| {
            paths.into_iter().map(|p| json!({"path":p.to_string_lossy(),"path_bytes":crate::presentation::path_bytes(p)})).collect::<Vec<_>>()
        };
        json!({"roots":paths(self.roots.iter().collect()),"exclusions":paths(self.exclusions.iter().collect())})
    }
}
impl ScopeArgs {
    pub fn resolve(&self, paths: &[PathBuf]) -> Result<Resolved, String> {
        let mut exclusions = self.exclude.clone();
        let roots = if let Some(name) = &self.scope {
            if !paths.is_empty() {
                return Err("explicit paths and --scope are mutually exclusive".into());
            }
            let config = self
                .workspace_config
                .clone()
                .or_else(|| {
                    std::env::var_os("HOME")
                        .map(|p| PathBuf::from(p).join(".config/workctl/workspaces.json"))
                })
                .ok_or("HOME unavailable; use --workspace-config")?;
            use std::io::Read;
            use std::os::unix::fs::OpenOptionsExt;
            let file = fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&config)
                .map_err(|e| format!("workspace configuration: {e}"))?;
            if !file.metadata().map_err(|e| e.to_string())?.is_file() {
                return Err("workspace configuration must be a regular file".into());
            }
            let mut bytes = Vec::new();
            file.take(65537)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 65536 {
                return Err("workspace configuration exceeds 64 KiB".into());
            }
            let mut config: Config =
                serde_json::from_slice(&bytes).map_err(|_| "invalid workspace configuration")?;
            if config.schema_version != 1 {
                return Err("unsupported workspace schema version".into());
            }
            let scope = config
                .scopes
                .remove(name)
                .ok_or("unknown workspace scope")?;
            if scope.roots.is_empty() {
                return Err("workspace scope has no roots".into());
            }
            if scope
                .roots
                .iter()
                .chain(&scope.exclusions)
                .any(|p| !p.is_absolute())
            {
                return Err("configured workspace paths must be absolute".into());
            }
            exclusions.extend(scope.exclusions);
            scope.roots
        } else {
            if self.workspace_config.is_some() {
                return Err("--workspace-config requires --scope".into());
            }
            if paths.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                paths.to_vec()
            }
        };
        let has_symlink_roots = roots
            .iter()
            .any(|p| fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()));
        let canonical = |p: PathBuf| {
            p.canonicalize().map_err(|e| {
                format!(
                    "{}: {e}",
                    crate::presentation::safe_text(&p.to_string_lossy())
                )
            })
        };
        let roots = roots
            .into_iter()
            .map(canonical)
            .collect::<Result<BTreeSet<_>, _>>()?
            .into_iter()
            .collect();
        let exclusions = exclusions
            .into_iter()
            .map(canonical)
            .collect::<Result<_, _>>()?;
        Ok(Resolved {
            roots,
            exclusions,
            has_symlink_roots,
        })
    }
}
pub(crate) fn duration(value: &str) -> Result<Duration, String> {
    let (number, multiplier) = match value.as_bytes().last() {
        Some(b's') => (&value[..value.len() - 1], 1u64),
        Some(b'm') => (&value[..value.len() - 1], 60),
        Some(b'h') => (&value[..value.len() - 1], 3600),
        Some(b'd') => (&value[..value.len() - 1], 86400),
        Some(b'w') => (&value[..value.len() - 1], 604800),
        _ => return Err("expected positive duration such as 7d (s, m, h, d, w)".into()),
    };
    if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
        return Err("duration must be a positive integer and unit".into());
    }
    let seconds = number
        .parse::<u64>()
        .ok()
        .and_then(|n| n.checked_mul(multiplier))
        .filter(|n| *n > 0)
        .ok_or("duration is zero or overflows")?;
    Ok(Duration::from_secs(seconds))
}
#[derive(Clone)]
pub(crate) struct Age {
    pub created: Option<SystemTime>,
    pub error: Option<String>,
}
pub(crate) struct CreationStamp {
    pub age: Age,
    pub identity: Option<(u64, u64)>,
}
impl CreationStamp {
    pub fn read(path: &Path) -> Self {
        Self::from_metadata(fs::symlink_metadata(path))
    }
    pub(crate) fn from_metadata(metadata: std::io::Result<fs::Metadata>) -> Self {
        match metadata {
            Ok(metadata) => Self {
                identity: Some((metadata.dev(), metadata.ino())),
                age: match metadata.created() {
                    Ok(created) => Age {
                        created: Some(created),
                        error: None,
                    },
                    Err(e) => Age {
                        created: None,
                        error: Some(e.to_string()),
                    },
                },
            },
            Err(e) => Self {
                identity: None,
                age: Age {
                    created: None,
                    error: Some(e.to_string()),
                },
            },
        }
    }
}
impl Age {
    pub fn read(path: &Path) -> Self {
        CreationStamp::read(path).age
    }
    pub fn older(&self, cutoff: SystemTime) -> bool {
        self.created.is_some_and(|created| created < cutoff)
    }
    pub fn evidence(&self) -> Value {
        json!({"created_unix_seconds":self.created.and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs_f64()),"source":if self.created.is_some(){"filesystem_creation_time"}else{"unknown"},"error":self.error})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creation_age_and_identity_share_the_selected_metadata_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("selected");
        fs::create_dir(&path).unwrap();
        let original = fs::symlink_metadata(&path).unwrap();
        let created = original.created().ok();
        let identity = (original.dev(), original.ino());
        fs::rename(&path, temp.path().join("saved")).unwrap();
        fs::create_dir(&path).unwrap();
        let stamp = CreationStamp::from_metadata(Ok(original));
        assert_eq!(stamp.age.created, created);
        assert_eq!(stamp.identity, Some(identity));
        let replacement = fs::symlink_metadata(&path).unwrap();
        assert_ne!(stamp.identity, Some((replacement.dev(), replacement.ino())));
    }
    #[test]
    fn duration_and_strict_creation_boundary() {
        assert_eq!(duration("7d").unwrap().as_secs(), 604800);
        for invalid in ["", "0d", "-1d", "1.5d", "7", "1D", "99999999999999999999w"] {
            assert!(duration(invalid).is_err(), "{invalid}");
        }
        let cutoff = UNIX_EPOCH + Duration::from_secs(100);
        for (created, expected) in [
            (None, false),
            (Some(cutoff), false),
            (Some(cutoff + Duration::from_nanos(1)), false),
            (Some(cutoff - Duration::from_nanos(1)), true),
        ] {
            assert_eq!(
                Age {
                    created,
                    error: None
                }
                .older(cutoff),
                expected
            );
        }
    }
}
