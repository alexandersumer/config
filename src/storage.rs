//! Opt-in allocation estimates; never used to authorize deletion.
use serde_json::{json, Value};
use std::collections::HashSet;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::{Duration, Instant};
pub(crate) fn measure(path: &Path) -> Value {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut allocated = 0u64;
    let mut estimated = 0u64;
    let mut logical = 0u64;
    let mut seen = HashSet::new();
    let mut errors = Vec::new();
    for entry in walkdir::WalkDir::new(path).follow_links(false) {
        if crate::runtime::cancelled() || Instant::now() >= deadline {
            errors.push("storage scan interrupted or exceeded 30 seconds".into());
            break;
        }
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                errors.push(e.to_string());
                continue;
            }
        };
        let metadata = match std::fs::symlink_metadata(entry.path()) {
            Ok(m) => m,
            Err(e) => {
                errors.push(e.to_string());
                continue;
            }
        };
        if !seen.insert((metadata.dev(), metadata.ino())) {
            continue;
        }
        let bytes = metadata.blocks().saturating_mul(512);
        allocated = allocated.saturating_add(bytes);
        logical = logical.saturating_add(metadata.len());
        // Shared hardlinks and symlink destinations cannot be claimed as freed.
        if metadata.is_dir() || metadata.nlink() == 1 {
            estimated = estimated.saturating_add(bytes);
        }
    }
    json!({"allocated_bytes":allocated,"logical_bytes":logical,"estimated_reclaimable_bytes":estimated,"measured_free_space_change_bytes":null,"complete":errors.is_empty(),"errors":errors,"note":"Allocation estimate only; APFS clones, snapshots, compression and hard links can change actual reclamation."})
}

pub(crate) fn available(path: &Path) -> Option<u128> {
    let path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).ok()?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    if unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) } != 0 {
        return None;
    }
    let stats = unsafe { stats.assume_init() };
    Some(stats.f_bavail as u128 * stats.f_frsize as u128)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sparse_files_hardlinks_and_symlinks_are_labeled_and_not_double_counted() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("sparse");
        std::fs::File::create(&file)
            .unwrap()
            .set_len(10_000_000)
            .unwrap();
        std::fs::hard_link(&file, temp.path().join("hardlink")).unwrap();
        symlink("/", temp.path().join("external")).unwrap();
        let measured = measure(temp.path());
        assert_eq!(measured["complete"], true);
        assert!(measured["allocated_bytes"].as_u64().unwrap() < 1_000_000);
        assert!(measured["logical_bytes"].as_u64().unwrap() < 11_000_000);
        assert!(measured["measured_free_space_change_bytes"].is_null());
        assert_eq!(measure(&temp.path().join("missing"))["complete"], false);
    }
}
