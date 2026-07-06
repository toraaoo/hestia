//! On-disk records of live supervised processes — `<dir>/<id>/record.json` is
//! the "re-adopt me" marker the next daemon recovers from. A record exists
//! exactly while its process is (believed) running.

use std::fs;
use std::path::Path;

use proto::process::ProcessSpec;
use serde::{Deserialize, Serialize};

use super::identity;

const FILE: &str = "record.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessRecord {
    pub id: String,
    pub pid: u32,
    pub pid_started: u64,
    pub spec: ProcessSpec,
    pub started_unix: i64,
}

impl ProcessRecord {
    pub fn for_spawn(spec: &ProcessSpec, pid: u32, started_unix: i64) -> Self {
        ProcessRecord {
            id: spec.id.clone(),
            pid,
            pid_started: identity::identify(pid).unwrap_or(0),
            spec: spec.clone(),
            started_unix,
        }
    }
}

pub fn save(proc_dir: &Path, record: &ProcessRecord) {
    let result = serde_json::to_vec_pretty(record)
        .map_err(std::io::Error::other)
        .and_then(|json| write_private(&proc_dir.join(FILE), &json));
    if let Err(e) = result {
        tracing::warn!(id = %record.id, "cannot persist process record: {e}");
    }
}

// The spec can carry launch credentials in its args, so the record is
// owner-only, like accounts.json.
#[cfg(unix)]
fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(contents)
}

#[cfg(not(unix))]
fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    fs::write(path, contents)
}

pub fn remove(proc_dir: &Path) {
    let _ = fs::remove_file(proc_dir.join(FILE));
}

pub fn scan(dir: &Path) -> Vec<ProcessRecord> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut records = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path().join(FILE);
        let Ok(contents) = fs::read(&path) else {
            continue;
        };
        match serde_json::from_slice::<ProcessRecord>(&contents) {
            Ok(record) => records.push(record),
            Err(e) => tracing::warn!(path = %path.display(), "discarding malformed record: {e}"),
        }
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("hestia-records-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn save_scan_remove_round_trip() {
        let dir = temp_dir("round-trip");
        let spec = ProcessSpec {
            id: "server-x".into(),
            program: "java".into(),
            ..Default::default()
        };
        let proc_dir = dir.join(&spec.id);
        fs::create_dir_all(&proc_dir).unwrap();
        save(
            &proc_dir,
            &ProcessRecord::for_spawn(&spec, std::process::id(), 42),
        );

        let scanned = scan(&dir);
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].id, "server-x");
        assert_eq!(scanned[0].started_unix, 42);
        assert!(identity::is_same(scanned[0].pid, scanned[0].pid_started));

        remove(&proc_dir);
        assert!(scan(&dir).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_skips_malformed_records() {
        let dir = temp_dir("malformed");
        let proc_dir = dir.join("broken");
        fs::create_dir_all(&proc_dir).unwrap();
        fs::write(proc_dir.join(FILE), b"not json").unwrap();
        assert!(scan(&dir).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
