//! On-disk records of supervised processes. `<dir>/<id>/record.json` is the
//! "re-adopt me" marker the next daemon recovers from, and exists exactly while
//! its process is (believed) running. When the process ends, that record is
//! replaced by a **tombstone** (`exit.json`) saying how it ended.
//!
//! The tombstone exists because absence could not carry that meaning. A finished
//! process used to leave an *unlabelled* directory — logs, no record — which is
//! indistinguishable from a stray a crash or a hand-edit left behind, and the
//! startup sweep deleted exactly those. So the daemon promised both "a terminal
//! state keeps its logs for post-mortem" and "the sweep deletes recordless
//! dirs", and the second quietly ate the first at every restart. Labelling the
//! end state makes the two rules describable at once: a directory with neither
//! marker is a true stray, and everything else is retained until pruned.

use std::fs;
use std::path::Path;

use proto::process::{ProcessSpec, ProcessState};
use serde::{Deserialize, Serialize};

use super::identity;

const FILE: &str = "record.json";
const TOMBSTONE: &str = "exit.json";

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

/// How a process ended, kept beside its logs so a post-mortem survives the
/// daemon that watched it.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tombstone {
    pub id: String,
    pub pid: u32,
    pub state: ProcessState,
    /// `None` for an adopted process — the exit code of a process we did not
    /// spawn is unknowable, a documented gap rather than a missing field.
    pub exit_code: Option<i32>,
    pub program: String,
    pub args: Vec<String>,
    pub started_unix: i64,
    pub ended_unix: i64,
    /// Where its output ended up — the reason the directory is kept at all, and
    /// not derivable without the spec, which dies with the record.
    pub log_path: std::path::PathBuf,
    #[serde(default)]
    pub err_path: Option<std::path::PathBuf>,
}

pub fn remove(proc_dir: &Path) {
    let _ = fs::remove_file(proc_dir.join(FILE));
}

/// Replace the live record with a tombstone: the process is over, and the
/// directory now says so rather than merely lacking a record.
pub fn entomb(proc_dir: &Path, tombstone: &Tombstone) {
    let result = serde_json::to_vec_pretty(tombstone)
        .map_err(std::io::Error::other)
        .and_then(|json| write_private(&proc_dir.join(TOMBSTONE), &json));
    if let Err(e) = result {
        tracing::warn!(id = %tombstone.id, "cannot persist process tombstone: {e}");
    }
    remove(proc_dir);
}

pub fn load_tombstone(proc_dir: &Path) -> Option<Tombstone> {
    let contents = fs::read(proc_dir.join(TOMBSTONE)).ok()?;
    serde_json::from_slice(&contents).ok()
}

/// Every tombstoned process directory under `dir`, newest first.
pub fn scan_tombstones(dir: &Path) -> Vec<Tombstone> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut tombstones: Vec<Tombstone> = entries
        .flatten()
        .filter_map(|entry| load_tombstone(&entry.path()))
        .collect();
    tombstones.sort_by_key(|t| std::cmp::Reverse(t.ended_unix));
    tombstones
}

/// Whether this directory belongs to a process at all — live or finished. A
/// directory with neither marker is a stray the sweep may delete.
pub fn is_known(proc_dir: &Path) -> bool {
    proc_dir.join(FILE).is_file() || proc_dir.join(TOMBSTONE).is_file()
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

    fn tombstone(id: &str, ended_unix: i64) -> Tombstone {
        Tombstone {
            id: id.to_string(),
            pid: 1234,
            state: ProcessState::Exited,
            exit_code: Some(0),
            program: "java".into(),
            args: Vec::new(),
            started_unix: ended_unix - 60,
            ended_unix,
            log_path: Path::new("/tmp/out.log").to_path_buf(),
            err_path: None,
        }
    }

    #[test]
    fn a_tombstone_labels_the_end_so_the_dir_is_not_a_stray() {
        let dir = temp_dir("tombstone");
        let proc_dir = dir.join("server-x");
        fs::create_dir_all(&proc_dir).unwrap();

        // A directory with neither marker is a stray: nothing claims it.
        assert!(!is_known(&proc_dir));

        let spec = ProcessSpec {
            id: "server-x".into(),
            program: "java".into(),
            ..Default::default()
        };
        save(&proc_dir, &ProcessRecord::for_spawn(&spec, 1234, 100));
        assert!(is_known(&proc_dir), "a live record claims it");

        entomb(&proc_dir, &tombstone("server-x", 200));
        assert!(
            is_known(&proc_dir),
            "and so does a tombstone — the sweep must not take a finished process's logs"
        );
        assert!(scan(&dir).is_empty(), "it is no longer a live record");

        let found = load_tombstone(&proc_dir).expect("the end state is readable");
        assert_eq!(found.state, ProcessState::Exited);
        assert_eq!(found.exit_code, Some(0));
        assert_eq!(found.log_path, Path::new("/tmp/out.log"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn tombstones_scan_newest_first() {
        let dir = temp_dir("tombstone-order");
        for (id, ended) in [("a", 300), ("b", 100), ("c", 200)] {
            let proc_dir = dir.join(id);
            fs::create_dir_all(&proc_dir).unwrap();
            entomb(&proc_dir, &tombstone(id, ended));
        }
        // Retention prunes from the end, so the order is the policy.
        let ids: Vec<String> = scan_tombstones(&dir).into_iter().map(|t| t.id).collect();
        assert_eq!(ids, ["a", "c", "b"]);
        fs::remove_dir_all(&dir).ok();
    }
}
