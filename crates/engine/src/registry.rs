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

/// Allocate a stable entry id: the name's slug tagged with a short random
/// suffix (`smp-3f9a2c7d`), retried against `taken` on the astronomically rare
/// clash. The slug keeps the id legible on disk and in logs; the suffix is what
/// makes the id unique and *stable*, so a rename is a metadata write — it never
/// has to move the directory or re-key the process. The id stays `[a-z0-9-]`,
/// never `_`, which the session-key scheme (`instance-<id>_<seq>`) reserves.
pub(crate) fn allocate_id(name: &str, taken: impl Fn(&str) -> bool) -> Result<String> {
    for _ in 0..8 {
        let id = format!("{}-{}", slugify(name)?, short_tag());
        if !taken(&id) {
            return Ok(id);
        }
    }
    bail!("could not allocate a unique id for '{name}'");
}

/// Eight hex chars from the random tail of a UUIDv7 (not its time prefix, so
/// ids minted in the same millisecond do not share a tag).
fn short_tag() -> String {
    Uuid::now_v7().simple().to_string()[24..32].to_string()
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
/// `proto::naming`). The slug prefixes an id but is no longer an id on its own —
/// [`allocate_id`] tags it with a stable suffix.
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
