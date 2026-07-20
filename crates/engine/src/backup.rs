//! Server backups: gzipped tar archives of a server's `data/` directory
//! under the entry root's `backups/` (appearing on demand, like the other
//! managed content directories). An archive is named
//! `<utc-stamp>-<kind>.tar.gz` — the disk is the registry, as with the entry
//! stores. Creation skips the top-level names the caller can re-materialise
//! (jar, libraries, logs); restore swaps the archive back in, carrying those
//! same names over from the current tree.

use std::collections::HashSet;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use proto::backup::{BackupInfo, BackupKind};
use serde::{Deserialize, Serialize};

const BACKUPS: &str = "backups";
const EXTENSION: &str = ".tar.gz";
const SESSION_LOCK: &str = "session.lock";

/// Progress for an archive/restore pass: `(files done, files total)`; total
/// is 0 when unknown (restoring streams the archive once).
pub type OnProgress<'a> = &'a (dyn Fn(u64, u64) + Send + Sync);

/// The reserved per-server scheduled-backup setting keys.
pub const INTERVAL_KEY: &str = "backup-interval";
pub const RETENTION_KEY: &str = "backup-retention";

/// Scheduled archives kept when no `backup-retention` is set.
pub const DEFAULT_RETENTION: usize = 7;

// A tighter schedule than this re-archives the world faster than it can
// meaningfully change and keeps the server's saving paused too often.
const MIN_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Per-server scheduled-backup tuning stored on the record: how often the
/// daemon archives the running server and how many scheduled archives to keep
/// (manual and pre-update backups are never pruned).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct BackupSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention: Option<u32>,
}

impl BackupSettings {
    /// The current value of a backup key, or `None` (outer) when `key` is not
    /// one of the reserved backup keys. The inner `None` means unset.
    pub fn get(&self, key: &str) -> Option<Option<String>> {
        match key {
            INTERVAL_KEY => Some(self.interval.clone()),
            RETENTION_KEY => Some(self.retention.map(|n| n.to_string())),
            _ => None,
        }
    }

    /// Apply a backup key. `Ok(false)` means `key` is not a backup key (fall
    /// through); an empty value clears the setting (an empty interval disables
    /// scheduled backups); an invalid value is `Err`.
    pub fn set(&mut self, key: &str, value: &str) -> Result<bool> {
        match key {
            INTERVAL_KEY => {
                self.interval = if value.trim().is_empty() {
                    None
                } else {
                    let normalized = value.trim().to_ascii_lowercase();
                    parse_interval(&normalized)?;
                    Some(normalized)
                };
                Ok(true)
            }
            RETENTION_KEY => {
                self.retention = if value.trim().is_empty() {
                    None
                } else {
                    let n: u32 = value
                        .trim()
                        .parse()
                        .ok()
                        .filter(|n| *n > 0)
                        .context("backup-retention must be a positive integer")?;
                    Some(n)
                };
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Both reserved keys with their current values (empty when unset), so a
    /// `config list` always shows what is settable.
    pub fn entries(&self) -> Vec<(String, String)> {
        vec![
            (
                INTERVAL_KEY.to_string(),
                self.interval.clone().unwrap_or_default(),
            ),
            (
                RETENTION_KEY.to_string(),
                self.retention.map(|n| n.to_string()).unwrap_or_default(),
            ),
        ]
    }

    /// The parsed schedule, `None` when scheduled backups are disabled (or the
    /// stored value no longer parses).
    pub fn interval(&self) -> Option<Duration> {
        self.interval
            .as_deref()
            .and_then(|v| parse_interval(v).ok())
    }

    pub fn retention(&self) -> usize {
        self.retention
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_RETENTION)
    }
}

/// Parse a schedule interval: digits followed by one unit char (m/h/d),
/// at least five minutes.
pub fn parse_interval(value: &str) -> Result<Duration> {
    let trimmed = value.trim();
    let unit_seconds = match trimmed.chars().last() {
        Some('m') => 60,
        Some('h') => 3600,
        Some('d') => 86400,
        _ => bail!("backup-interval must look like 30m, 6h, or 1d"),
    };
    let digits = &trimmed[..trimmed.len() - 1];
    let count: u64 = digits
        .parse()
        .ok()
        .filter(|n| *n > 0)
        .context("backup-interval must look like 30m, 6h, or 1d")?;
    let interval = Duration::from_secs(count.saturating_mul(unit_seconds));
    if interval < MIN_INTERVAL {
        bail!("backup-interval must be at least 5m");
    }
    Ok(interval)
}

pub fn backups_dir(entry_dir: &Path) -> PathBuf {
    entry_dir.join(BACKUPS)
}

/// Archive `data_dir` into a new backup under the entry root, skipping the
/// top-level `exclude` names and Minecraft's transient `session.lock` files.
/// The archive lands whole or not at all (written through a `.part` temp file).
pub fn create(
    entry_dir: &Path,
    data_dir: &Path,
    kind: BackupKind,
    exclude: &[String],
    on_progress: OnProgress<'_>,
) -> Result<BackupInfo> {
    if !data_dir.is_dir() {
        bail!("nothing to back up yet (no data directory)");
    }
    let dir = backups_dir(entry_dir);
    std::fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let id = allocate_id(entry_dir, kind);
    let path = dir.join(format!("{id}{EXTENSION}"));
    let part = dir.join(format!("{id}{EXTENSION}.part"));

    let written = write_archive(&part, data_dir, exclude, on_progress);
    if let Err(e) = written {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    std::fs::rename(&part, &path).with_context(|| format!("cannot finalise {}", path.display()))?;
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    tracing::info!(id, kind = kind.as_str(), size, "backup created");
    Ok(BackupInfo {
        id,
        kind,
        created_unix: now_unix(),
        size,
    })
}

fn write_archive(
    dest: &Path,
    data_dir: &Path,
    exclude: &[String],
    on_progress: OnProgress<'_>,
) -> Result<()> {
    let file = File::create(dest).with_context(|| format!("cannot create {}", dest.display()))?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);
    builder.follow_symlinks(false);

    let total = count_files(data_dir, exclude)?;
    let mut done = 0u64;
    for entry in included_roots(data_dir, exclude)? {
        append_tree(
            &mut builder,
            data_dir,
            &entry,
            &mut done,
            total,
            on_progress,
        )?;
    }
    builder
        .into_inner()
        .context("finalising the archive")?
        .finish()
        .context("flushing the archive")?
        .sync_all()
        .context("syncing the archive")?;
    Ok(())
}

/// The top-level names under `data_dir` a backup includes (everything not
/// excluded, skipping non-UTF-8 names — tar headers are byte-exact but the
/// exclude match is string-based, so an unmatchable name is safer skipped).
fn included_roots(data_dir: &Path, exclude: &[String]) -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    let entries = std::fs::read_dir(data_dir)
        .with_context(|| format!("cannot read {}", data_dir.display()))?;
    for entry in entries {
        let entry = entry.context("reading the data directory")?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            tracing::warn!(path = %entry.path().display(), "skipping non-UTF-8 name");
            continue;
        };
        if exclude.iter().any(|e| e == name) {
            continue;
        }
        roots.push(PathBuf::from(name));
    }
    roots.sort();
    Ok(roots)
}

fn count_files(data_dir: &Path, exclude: &[String]) -> Result<u64> {
    fn walk(path: &Path) -> u64 {
        if is_session_lock(path) {
            return 0;
        }
        if !path.is_dir() || path.is_symlink() {
            return 1;
        }
        std::fs::read_dir(path)
            .map(|entries| entries.flatten().map(|e| walk(&e.path())).sum())
            .unwrap_or(0)
    }
    Ok(included_roots(data_dir, exclude)?
        .iter()
        .map(|rel| walk(&data_dir.join(rel)))
        .sum())
}

/// Append `rel` (a path under `base`) file by file, so progress ticks per
/// file rather than per top-level directory.
fn append_tree(
    builder: &mut tar::Builder<flate2::write::GzEncoder<File>>,
    base: &Path,
    rel: &Path,
    done: &mut u64,
    total: u64,
    on_progress: OnProgress<'_>,
) -> Result<()> {
    let path = base.join(rel);
    if is_session_lock(&path) {
        return Ok(());
    }
    if path.is_symlink() || !path.is_dir() {
        builder
            .append_path_with_name(&path, rel)
            .with_context(|| format!("archiving {}", rel.display()))?;
        *done += 1;
        on_progress(*done, total);
        return Ok(());
    }
    builder
        .append_dir(rel, &path)
        .with_context(|| format!("archiving {}", rel.display()))?;
    let entries =
        std::fs::read_dir(&path).with_context(|| format!("cannot read {}", path.display()))?;
    let mut children: Vec<_> = entries
        .collect::<std::io::Result<Vec<_>>>()
        .context("reading a data subdirectory")?;
    children.sort_by_key(|e| e.file_name());
    for child in children {
        append_tree(
            builder,
            base,
            &rel.join(child.file_name()),
            done,
            total,
            on_progress,
        )?;
    }
    Ok(())
}

/// A world lock is transient state, not world data. It must not survive into
/// an archive, regardless of which world directory contains it.
fn is_session_lock(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == SESSION_LOCK)
}

/// Every stored backup, newest first.
pub fn list(entry_dir: &Path) -> Vec<BackupInfo> {
    let mut backups = Vec::new();
    if let Ok(entries) = std::fs::read_dir(backups_dir(entry_dir)) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(stem) = name.strip_suffix(EXTENSION) else {
                continue;
            };
            let Some(kind) = kind_of(stem) else { continue };
            let metadata = entry.metadata().ok();
            backups.push(BackupInfo {
                id: stem.to_string(),
                kind,
                created_unix: metadata
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
                size: metadata.map(|m| m.len()).unwrap_or(0),
            });
        }
    }
    backups.sort_by(|a, b| b.id.cmp(&a.id));
    backups
}

pub fn find(entry_dir: &Path, reference: &str) -> Option<BackupInfo> {
    list(entry_dir).into_iter().find(|b| b.id == reference)
}

/// Delete one backup archive. Returns false when no backup matches.
pub fn remove(entry_dir: &Path, reference: &str) -> Result<bool> {
    let Some(backup) = find(entry_dir, reference) else {
        return Ok(false);
    };
    let path = backups_dir(entry_dir).join(format!("{}{EXTENSION}", backup.id));
    std::fs::remove_file(&path).with_context(|| format!("cannot remove {}", path.display()))?;
    tracing::info!(id = %backup.id, "backup removed");
    Ok(true)
}

/// Replace `data_dir` with a backup's content. The archive extracts into a
/// staging directory first and the trees swap only when extraction succeeded,
/// so a failure leaves the current data untouched. The top-level `preserve`
/// names (the same set excluded at create) carry over from the current tree —
/// they belong to the entry's *current* version, not the backup's.
pub fn restore(
    entry_dir: &Path,
    data_dir: &Path,
    reference: &str,
    preserve: &[String],
    on_progress: OnProgress<'_>,
) -> Result<BackupInfo> {
    let backup =
        find(entry_dir, reference).with_context(|| format!("no backup matches '{reference}'"))?;
    let archive = backups_dir(entry_dir).join(format!("{}{EXTENSION}", backup.id));
    let staging = entry_dir.join(format!(".restore-{}", backup.id));
    if staging.exists() {
        std::fs::remove_dir_all(&staging).context("cannot clear a stale restore staging")?;
    }
    std::fs::create_dir_all(&staging)
        .with_context(|| format!("cannot create {}", staging.display()))?;

    let extracted = extract_archive(&archive, &staging, on_progress).and_then(|()| {
        for name in preserve {
            let current = data_dir.join(name);
            if !current.exists() {
                continue;
            }
            let kept = staging.join(name);
            if kept.exists() {
                if kept.is_dir() {
                    std::fs::remove_dir_all(&kept)
                } else {
                    std::fs::remove_file(&kept)
                }
                .with_context(|| format!("cannot drop the archived {name}"))?;
            }
            std::fs::rename(&current, &kept)
                .with_context(|| format!("cannot carry {name} over"))?;
        }
        Ok(())
    });
    if let Err(e) = extracted {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }

    let trash = entry_dir.join(format!(".discard-{}", backup.id));
    if trash.exists() {
        std::fs::remove_dir_all(&trash).context("cannot clear a stale discard directory")?;
    }
    if data_dir.exists() {
        std::fs::rename(data_dir, &trash).context("cannot set the current data aside")?;
    }
    if let Err(e) = std::fs::rename(&staging, data_dir) {
        let _ = std::fs::rename(&trash, data_dir);
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e).context("cannot move the restored data into place");
    }
    if let Err(e) = std::fs::remove_dir_all(&trash) {
        if trash.exists() {
            tracing::warn!(path = %trash.display(), error = %e, "replaced data left behind");
        }
    }
    tracing::info!(id = %backup.id, "backup restored");
    Ok(backup)
}

fn extract_archive(archive: &Path, dest: &Path, on_progress: OnProgress<'_>) -> Result<()> {
    let file = File::open(archive).with_context(|| format!("cannot open {}", archive.display()))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    tar.set_preserve_permissions(true);
    // unpack_in refuses entries that would escape `dest`.
    let mut done = 0u64;
    for entry in tar.entries().context("reading the archive")? {
        let mut entry = entry.context("reading an archive entry")?;
        entry
            .unpack_in(dest)
            .context("extracting an archive entry")?;
        done += 1;
        on_progress(done, 0);
    }
    Ok(())
}

/// Delete the oldest `kind` backups beyond `keep`; other kinds are untouched.
/// Returns the removed backups.
pub fn prune(entry_dir: &Path, kind: BackupKind, keep: usize) -> Result<Vec<BackupInfo>> {
    let expired: Vec<BackupInfo> = list(entry_dir)
        .into_iter()
        .filter(|b| b.kind == kind)
        .skip(keep)
        .collect();
    for backup in &expired {
        remove(entry_dir, &backup.id)?;
    }
    Ok(expired)
}

/// A fresh backup id: the current UTC time plus the kind, suffixed on the
/// (unlikely) same-second collision.
fn allocate_id(entry_dir: &Path, kind: BackupKind) -> String {
    let taken: HashSet<String> = list(entry_dir).into_iter().map(|b| b.id).collect();
    let base = format!("{}-{}", utc_stamp(now_unix()), kind.as_str());
    if !taken.contains(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !taken.contains(id))
        .expect("an unclaimed backup id exists")
}

/// The kind segment of a backup id — the rightmost dash-separated token that
/// names a kind (a collision suffix may follow it).
fn kind_of(stem: &str) -> Option<BackupKind> {
    stem.rsplit('-').find_map(BackupKind::parse)
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `YYYYMMDD-HHMMSS` in UTC (Howard Hinnant's civil-from-days algorithm; no
/// date-time dependency for one format).
fn utc_stamp(unix: i64) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_are_utc_civil_dates() {
        assert_eq!(utc_stamp(0), "19700101-000000");
        assert_eq!(utc_stamp(1_751_852_045), "20250707-013405");
    }

    #[test]
    fn intervals_parse_with_units() {
        assert_eq!(parse_interval("30m").unwrap(), Duration::from_secs(1800));
        assert_eq!(parse_interval("6h").unwrap(), Duration::from_secs(21_600));
        assert_eq!(parse_interval("1d").unwrap(), Duration::from_secs(86_400));
        assert!(parse_interval("4m").is_err());
        assert!(parse_interval("0h").is_err());
        assert!(parse_interval("h").is_err());
        assert!(parse_interval("90").is_err());
        assert!(parse_interval("soon").is_err());
    }

    fn temp_entry(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("hestia-backup-test-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("data")).unwrap();
        dir
    }

    fn write(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn create_restore_round_trips_and_preserves_excluded_paths() {
        let entry = temp_entry("round-trip");
        let data = entry.join("data");
        let exclude = vec!["server.jar".to_string(), "logs".to_string()];
        write(&data.join("world/level.dat"), "one");
        write(&data.join("server.properties"), "motd=hi");
        write(&data.join("server.jar"), "current-jar");
        write(&data.join("logs/latest.log"), "log");

        let backup = create(&entry, &data, BackupKind::Manual, &exclude, &|_, _| {}).unwrap();
        assert_eq!(list(&entry).len(), 1);
        assert_eq!(list(&entry)[0].id, backup.id);
        assert_eq!(list(&entry)[0].kind, BackupKind::Manual);

        write(&data.join("world/level.dat"), "two");
        write(&data.join("server.jar"), "newer-jar");
        std::fs::remove_file(data.join("server.properties")).unwrap();

        let restored = restore(&entry, &data, &backup.id, &exclude, &|_, _| {}).unwrap();
        assert_eq!(restored.id, backup.id);
        let content = |p: &str| std::fs::read_to_string(data.join(p)).unwrap();
        assert_eq!(content("world/level.dat"), "one");
        assert_eq!(content("server.properties"), "motd=hi");
        // Excluded paths carry over from the current tree, not the archive.
        assert_eq!(content("server.jar"), "newer-jar");
        assert_eq!(content("logs/latest.log"), "log");

        assert!(remove(&entry, &backup.id).unwrap());
        assert!(list(&entry).is_empty());
        std::fs::remove_dir_all(&entry).ok();
    }

    #[test]
    fn create_omits_session_locks_at_every_depth() {
        let entry = temp_entry("session-lock");
        let data = entry.join("data");
        write(&data.join("session.lock"), "root lock");
        write(&data.join("world/session.lock"), "world lock");
        write(&data.join("world/region/r.0.0.mca"), "world data");

        let backup = create(&entry, &data, BackupKind::Manual, &[], &|_, _| {}).unwrap();
        let archive = backups_dir(&entry).join(format!("{}{EXTENSION}", backup.id));
        let file = File::open(archive).unwrap();
        let decoder = flate2::read::GzDecoder::new(file);
        let mut tar = tar::Archive::new(decoder);
        let paths: Vec<PathBuf> = tar
            .entries()
            .unwrap()
            .map(|entry| entry.unwrap().path().unwrap().into_owned())
            .collect();

        assert!(paths
            .iter()
            .any(|path| path == Path::new("world/region/r.0.0.mca")));
        assert!(!paths
            .iter()
            .any(|path| path.file_name().is_some_and(|name| name == SESSION_LOCK)));
        std::fs::remove_dir_all(&entry).ok();
    }

    #[test]
    fn prune_drops_only_old_backups_of_the_kind() {
        let entry = temp_entry("prune");
        let data = entry.join("data");
        write(&data.join("world/level.dat"), "x");

        let old = create(&entry, &data, BackupKind::Scheduled, &[], &|_, _| {}).unwrap();
        let mid = create(&entry, &data, BackupKind::Scheduled, &[], &|_, _| {}).unwrap();
        let new = create(&entry, &data, BackupKind::Scheduled, &[], &|_, _| {}).unwrap();
        let manual = create(&entry, &data, BackupKind::Manual, &[], &|_, _| {}).unwrap();

        let removed = prune(&entry, BackupKind::Scheduled, 1).unwrap();
        let removed_ids: Vec<&str> = removed.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(removed_ids, vec![mid.id.as_str(), old.id.as_str()]);
        let mut kept: Vec<String> = list(&entry).into_iter().map(|b| b.id).collect();
        kept.sort();
        let mut expected = vec![manual.id.clone(), new.id.clone()];
        expected.sort();
        assert_eq!(kept, expected);
        std::fs::remove_dir_all(&entry).ok();
    }

    #[test]
    fn missing_data_and_unknown_backups_are_errors() {
        let entry = temp_entry("errors");
        let missing = entry.join("nope");
        assert!(create(&entry, &missing, BackupKind::Manual, &[], &|_, _| {}).is_err());
        assert!(restore(&entry, &entry.join("data"), "ghost", &[], &|_, _| {}).is_err());
        assert!(!remove(&entry, "ghost").unwrap());
        std::fs::remove_dir_all(&entry).ok();
    }

    #[test]
    fn settings_validate_and_round_trip_keys() {
        let mut settings = BackupSettings::default();
        assert!(!settings.set("motd", "hi").unwrap());
        assert!(settings.set(INTERVAL_KEY, "6H").unwrap());
        assert_eq!(settings.interval(), Some(Duration::from_secs(21_600)));
        assert!(settings.set(RETENTION_KEY, "3").unwrap());
        assert_eq!(settings.retention(), 3);
        assert!(settings.set(INTERVAL_KEY, "soon").is_err());
        assert!(settings.set(RETENTION_KEY, "0").is_err());
        assert!(settings.set(INTERVAL_KEY, "").unwrap());
        assert_eq!(settings.interval(), None);
        assert_eq!(settings.get(RETENTION_KEY), Some(Some("3".to_string())));
    }

    #[test]
    fn kinds_parse_from_ids() {
        assert_eq!(kind_of("20260707-120000-manual"), Some(BackupKind::Manual));
        assert_eq!(
            kind_of("20260707-120000-scheduled-2"),
            Some(BackupKind::Scheduled)
        );
        assert_eq!(kind_of("20260707-120000-update"), Some(BackupKind::Update));
        assert_eq!(kind_of("20260707-120000"), None);
    }
}
