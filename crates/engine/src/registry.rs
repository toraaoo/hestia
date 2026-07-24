//! Shared helpers for the disk-backed record stores (`servers`, `instances`):
//! each entry is a directory holding a JSON record, listing scans the parent —
//! the disk is the registry.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

pub(crate) fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// A stable, opaque entry id (UUIDv7 hex): the entry's internal key — process
/// key, port/in-flight claims, process records — never a path component, so a
/// rename never touches it. Stays `[0-9a-f]`, never `_` (the `<id>_<seq>`
/// session-key scheme reserves it).
pub(crate) fn allocate_id(taken: impl Fn(&str) -> bool) -> Result<String> {
    for _ in 0..8 {
        let id = Uuid::now_v7().simple().to_string();
        if !taken(&id) {
            return Ok(id);
        }
    }
    bail!("could not allocate a unique entry id");
}

/// An entry's directory name: its display name slugged (unique via
/// [`name_taken`]), so a rename moves the directory. Falls back to the id for a
/// name with no sluggable characters, which create forbids.
pub(crate) fn dir_name(id: &str, name: &str) -> String {
    proto::naming::slugify(name).unwrap_or_else(|| id.to_string())
}

/// True when `name` collides with an existing entry's display name once both
/// are slugged — two entries must not reduce to the same slug, or a bare-name
/// reference would be ambiguous (`Modded` and `modded` are the same entry).
pub(crate) fn name_taken<'a>(name: &str, existing: impl IntoIterator<Item = &'a str>) -> bool {
    let Ok(slug) = slugify(name) else {
        return false;
    };
    existing
        .into_iter()
        .any(|other| slugify(other).map(|s| s == slug).unwrap_or(false))
}

/// Reduce a display name to a filesystem-safe slug (the shared rule lives in
/// `proto::naming`); it names the entry's directory ([`dir_name`]).
pub(crate) fn slugify(name: &str) -> Result<String> {
    match proto::naming::slugify(name) {
        Some(slug) => Ok(slug),
        None => bail!("name '{name}' has no usable characters"),
    }
}

pub(crate) fn read_record<T: DeserializeOwned>(dir: &Path, file: &str) -> Option<T> {
    let text = std::fs::read_to_string(dir.join(file)).ok()?;
    serde_json::from_str(&text).ok()
}

pub(crate) fn write_record<T: Serialize>(dir: &Path, file: &str, record: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(record).context("record serializes")?;
    std::fs::write(dir.join(file), format!("{text}\n"))
        .with_context(|| format!("cannot write {file} in {}", dir.display()))
}

/// Every record under `dir` (one subdirectory per entry, skipping any that has
/// no readable record).
pub(crate) fn scan<T: DeserializeOwned>(dir: &Path, file: &str) -> Vec<T> {
    let mut records = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(record) = read_record(&entry.path(), file) {
                    records.push(record);
                }
            }
        }
    }
    records
}
