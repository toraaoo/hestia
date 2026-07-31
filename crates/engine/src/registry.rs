//! Shared helpers for the disk-backed record stores (`servers`, `instances`):
//! each entry is a directory holding a JSON record, listing scans the parent —
//! the disk is the registry.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Result};
use uuid::Uuid;

use crate::schema::{self, Document};

pub(crate) fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `YYYYMMDD-HHMMSS` in UTC (Howard Hinnant's civil-from-days algorithm; no
/// date-time dependency for one format). What names a backup archive and an
/// exported instance — anything the disk registry sorts by time.
pub(crate) fn utc_stamp(unix: i64) -> String {
    let days = unix.div_euclid(86400);
    let secs = unix.rem_euclid(86400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
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

/// Where a record lives: its directory plus the file name the document type
/// itself declares, so the two can never be paired wrongly at a call site.
pub(crate) fn record_path<T: Document>(dir: &Path) -> PathBuf {
    dir.join(T::NAME)
}

pub(crate) fn read_record<T: Document>(dir: &Path) -> Option<T> {
    schema::load(&record_path::<T>(dir))
}

pub(crate) fn write_record<T: Document>(dir: &Path, record: &T) -> Result<()> {
    schema::save(&record_path::<T>(dir), record)
}

/// Every record under `dir` (one subdirectory per entry, skipping any that has
/// no readable record — an unreadable one is set aside and reported by
/// [`schema::load`] rather than passed off as absent).
pub(crate) fn scan<T: Document>(dir: &Path) -> Vec<T> {
    let mut records = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(record) = read_record(&entry.path()) {
                    records.push(record);
                }
            }
        }
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_are_utc_civil_dates() {
        assert_eq!(utc_stamp(0), "19700101-000000");
        assert_eq!(utc_stamp(1_751_852_045), "20250707-013405");
    }
}
