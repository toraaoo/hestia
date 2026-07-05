//! Minecraft-style rotating log file. The active log is always `latest.log`; on
//! startup and whenever it grows past `max_bytes` it is gzip-compressed to a dated
//! archive (`<stem>-YYYY-MM-DD-N.log.gz`) and a fresh `latest.log` begins. Archives
//! beyond `keep` are pruned oldest-first.
//!
//! This writer runs on tracing-appender's non-blocking worker thread, so the
//! blocking compression on rotation never stalls an application thread.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::write::GzEncoder;
use flate2::Compression;

/// The fixed name of the live log; the archives carry the dated names.
pub const ACTIVE_NAME: &str = "latest.log";

pub struct RollingLog {
    dir: PathBuf,
    stem: String,
    max_bytes: u64,
    keep: usize,
    file: File,
    written: u64,
}

impl RollingLog {
    /// Open the live log, first archiving any leftover `latest.log` from a previous
    /// run so each run starts a fresh file.
    pub fn open(dir: PathBuf, stem: &str, max_bytes: u64, keep: usize) -> io::Result<Self> {
        fs::create_dir_all(&dir)?;
        let active = dir.join(ACTIVE_NAME);
        if fs::metadata(&active).map(|m| m.len() > 0).unwrap_or(false) {
            let _ = archive(&dir, stem, &active);
        }
        let file = open_active(&active)?;
        let log = RollingLog {
            dir,
            stem: stem.to_string(),
            max_bytes,
            keep,
            file,
            written: 0,
        };
        log.prune();
        Ok(log)
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.file.flush()?;
        let active = self.dir.join(ACTIVE_NAME);
        archive(&self.dir, &self.stem, &active)?;
        self.file = open_active(&active)?;
        self.written = 0;
        self.prune();
        Ok(())
    }

    fn prune(&self) {
        let mut archives: Vec<PathBuf> = match fs::read_dir(&self.dir) {
            Ok(entries) => entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| is_archive(p, &self.stem))
                .collect(),
            Err(_) => return,
        };
        if archives.len() <= self.keep {
            return;
        }
        archives.sort_by_key(|p| fs::metadata(p).and_then(|m| m.modified()).ok());
        for old in &archives[..archives.len() - self.keep] {
            let _ = fs::remove_file(old);
        }
    }
}

impl Write for RollingLog {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.written >= self.max_bytes {
            let _ = self.rotate();
        }
        let n = self.file.write(buf)?;
        self.written += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

fn open_active(active: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(active)
}

fn archive(dir: &Path, stem: &str, active: &Path) -> io::Result<()> {
    let dest = dir.join(next_archive_name(dir, stem));
    let mut input = File::open(active)?;
    let mut encoder = GzEncoder::new(File::create(&dest)?, Compression::default());
    io::copy(&mut input, &mut encoder)?;
    encoder.finish()?;
    Ok(())
}

/// The next free `<stem>-<date>-<n>.log.gz` for today, so multiple rotations in a
/// single day never collide.
fn next_archive_name(dir: &Path, stem: &str) -> String {
    let prefix = format!("{stem}-{}-", utc_date(SystemTime::now()));
    let mut next = 1;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if let Some(index) = name
                .to_str()
                .and_then(|n| n.strip_prefix(&prefix))
                .and_then(|rest| rest.strip_suffix(".log.gz"))
                .and_then(|n| n.parse::<u32>().ok())
            {
                next = next.max(index + 1);
            }
        }
    }
    format!("{prefix}{next}.log.gz")
}

fn is_archive(path: &Path, stem: &str) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with(&format!("{stem}-")) && n.ends_with(".log.gz"))
        .unwrap_or(false)
}

fn utc_date(now: SystemTime) -> String {
    let secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Howard Hinnant's civil-from-days: a Unix day number to a UTC calendar date via
/// pure integer math (no timezone or leap tables). Matches tracing's UTC event
/// timestamps.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn date_of(epoch_secs: u64) -> String {
        utc_date(UNIX_EPOCH + Duration::from_secs(epoch_secs))
    }

    #[test]
    fn utc_date_matches_known_epochs() {
        assert_eq!(date_of(0), "1970-01-01");
        assert_eq!(date_of(1_704_067_200), "2024-01-01");
        assert_eq!(date_of(1_709_164_800), "2024-02-29");
        assert_eq!(date_of(1_735_689_600), "2025-01-01");
    }

    #[test]
    fn open_archives_previous_run_and_prunes() {
        let dir = std::env::temp_dir().join(format!("hestia-rolling-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join(ACTIVE_NAME), b"previous run\n").unwrap();
        {
            let mut log = RollingLog::open(dir.clone(), "test", 1024, 2).unwrap();
            log.write_all(b"fresh run\n").unwrap();
            log.flush().unwrap();
        }

        let archives: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| is_archive(p, "test"))
            .collect();
        assert_eq!(archives.len(), 1, "previous latest.log should be archived");
        assert_eq!(
            fs::read_to_string(dir.join(ACTIVE_NAME)).unwrap(),
            "fresh run\n"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotation_compresses_when_over_limit() {
        let dir = std::env::temp_dir().join(format!("hestia-rotate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);

        let mut log = RollingLog::open(dir.clone(), "test", 16, 4).unwrap();
        for _ in 0..5 {
            log.write_all(b"0123456789ABCDEF0123456789\n").unwrap();
            log.flush().unwrap();
        }

        let archives = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| is_archive(p, "test"))
            .count();
        assert!(archives >= 1, "expected at least one compressed archive");

        let _ = fs::remove_dir_all(&dir);
    }
}
