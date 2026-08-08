//! Where a node's key is kept: the OS keyring when one answers, else an
//! owner-only file in the data home. The fallback exists because a control plane
//! is often a headless box with no Secret Service, but it is weaker — a synced
//! data home carries the keys with it — so the keyring is tried first.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The keyring service every node's key is filed under.
const SERVICE: &str = "hestia-node";

#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Fallback {
    keys: BTreeMap<String, String>,
}

pub fn store(home: &Path, node_id: &str, token: &str) -> Result<(), String> {
    match keyring::Entry::new(SERVICE, node_id).and_then(|e| e.set_password(token)) {
        Ok(()) => Ok(()),
        Err(e) => {
            tracing::warn!(
                node = node_id,
                "no keyring available ({e}); keeping the key in an owner-only file"
            );
            let mut fallback = read_fallback(home);
            fallback.keys.insert(node_id.to_string(), token.to_string());
            write_fallback(home, &fallback)
        }
    }
}

pub fn read(home: &Path, node_id: &str) -> Option<String> {
    if let Ok(token) = keyring::Entry::new(SERVICE, node_id).and_then(|e| e.get_password()) {
        return Some(token);
    }
    read_fallback(home).keys.remove(node_id)
}

/// Forget a node's key, wherever it ended up. Both stores are cleared: a key
/// that moved between them must not survive in the one nobody looked at.
pub fn forget(home: &Path, node_id: &str) {
    if let Ok(entry) = keyring::Entry::new(SERVICE, node_id) {
        let _ = entry.delete_credential();
    }
    let mut fallback = read_fallback(home);
    if fallback.keys.remove(node_id).is_some() {
        let _ = write_fallback(home, &fallback);
    }
}

fn fallback_path(home: &Path) -> PathBuf {
    home.join("node-keys.json")
}

fn read_fallback(home: &Path) -> Fallback {
    std::fs::read_to_string(fallback_path(home))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_fallback(home: &Path, fallback: &Fallback) -> Result<(), String> {
    let path = fallback_path(home);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(fallback).map_err(|e| e.to_string())?;
    // Mode set on the temp before the rename: never briefly world-readable.
    let part = path.with_extension("json.part");
    std::fs::write(&part, format!("{text}\n")).map_err(|e| e.to_string())?;
    restrict(&part).map_err(|e| e.to_string())?;
    std::fs::rename(&part, &path).map_err(|e| e.to_string())
}

#[cfg(unix)]
fn restrict(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix("hestia-secrets-")
            .tempdir()
            .expect("temp dir")
    }

    #[test]
    fn a_key_written_to_the_fallback_reads_back() {
        let dir = temp();
        let mut fallback = Fallback::default();
        fallback.keys.insert("n1".into(), "hst_secret".into());
        write_fallback(dir.path(), &fallback).expect("write");

        assert_eq!(
            read_fallback(dir.path()).keys.get("n1").map(String::as_str),
            Some("hst_secret")
        );
    }

    #[test]
    fn the_fallback_is_owner_only() {
        let dir = temp();
        write_fallback(dir.path(), &Fallback::default()).expect("write");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(fallback_path(dir.path()))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o077, 0, "group and other must have no access");
        }
        assert!(fallback_path(dir.path()).exists());
    }

    #[test]
    fn a_write_leaves_no_temp_behind() {
        let dir = temp();
        write_fallback(dir.path(), &Fallback::default()).expect("write");

        let names: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["node-keys.json"]);
    }

    #[test]
    fn forgetting_a_node_clears_the_fallback_too() {
        let dir = temp();
        let mut fallback = Fallback::default();
        fallback.keys.insert("n1".into(), "hst_secret".into());
        fallback.keys.insert("n2".into(), "hst_other".into());
        write_fallback(dir.path(), &fallback).expect("write");

        forget(dir.path(), "n1");

        let left = read_fallback(dir.path()).keys;
        assert!(!left.contains_key("n1"));
        assert!(left.contains_key("n2"), "the others are left alone");
    }
}
