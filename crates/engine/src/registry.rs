//! Shared helpers for the disk-backed record stores (`servers`, `instances`):
//! each entry is a directory holding a JSON record, listing scans the parent —
//! the disk is the registry.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub(crate) fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Reduce a display name to a filesystem-safe id: lowercase alphanumeric runs
/// joined by single dashes.
pub(crate) fn slugify(name: &str) -> Result<String> {
    let mut slug = String::new();
    let mut gap = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if gap && !slug.is_empty() {
                slug.push('-');
            }
            gap = false;
            slug.push(c.to_ascii_lowercase());
        } else {
            gap = true;
        }
    }
    if slug.is_empty() {
        bail!("name '{name}' has no usable characters");
    }
    Ok(slug)
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
