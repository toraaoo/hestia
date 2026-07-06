//! Process-wide logging configuration (tracing). `LogLevel` is Hestia's own enum
//! so callers don't depend on tracing's types. Each sink has its own level: the
//! console (stderr) and an optional rotating, compressed file (see [`FileLog`]) —
//! the daemon's fresh-per-run `latest.log`, or the appended `<stem>.log` that
//! short-lived clients share so their diagnostics never land on the console.

use std::io;
use std::path::PathBuf;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::writer::{BoxMakeWriter, MakeWriterExt};
use tracing_subscriber::EnvFilter;

use crate::rolling::{RollingLog, ACTIVE_NAME};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Off,
}

impl Default for LogLevel {
    /// The level used when none is requested: `Trace` in debug builds so a
    /// development run captures everything, `Info` in release.
    fn default() -> Self {
        if cfg!(debug_assertions) {
            LogLevel::Trace
        } else {
            LogLevel::Info
        }
    }
}

impl LogLevel {
    fn filter(self) -> LevelFilter {
        match self {
            LogLevel::Trace => LevelFilter::TRACE,
            LogLevel::Debug => LevelFilter::DEBUG,
            LogLevel::Info => LevelFilter::INFO,
            LogLevel::Warn => LevelFilter::WARN,
            LogLevel::Error => LevelFilter::ERROR,
            LogLevel::Off => LevelFilter::OFF,
        }
    }

    fn as_level(self) -> Option<tracing::Level> {
        match self {
            LogLevel::Trace => Some(tracing::Level::TRACE),
            LogLevel::Debug => Some(tracing::Level::DEBUG),
            LogLevel::Info => Some(tracing::Level::INFO),
            LogLevel::Warn => Some(tracing::Level::WARN),
            LogLevel::Error => Some(tracing::Level::ERROR),
            LogLevel::Off => None,
        }
    }
}

const DEFAULT_MAX_BYTES: u64 = 10 * 1024 * 1024;
const DEFAULT_KEEP: usize = 8;

/// A rotating file log sink with its own level, rotated to dated gzip archives
/// (`<stem>-YYYY-MM-DD-N.log.gz`) once the live file exceeds `max_bytes`,
/// keeping the newest `keep` archives.
pub struct FileLog {
    dir: PathBuf,
    stem: String,
    level: LogLevel,
    max_bytes: u64,
    keep: usize,
    append: bool,
}

impl FileLog {
    /// A fresh-per-run `latest.log`, archiving any leftover on startup — for a
    /// long-lived process like the daemon, where one run is one file.
    pub fn new(dir: impl Into<PathBuf>, stem: impl Into<String>, level: LogLevel) -> Self {
        FileLog {
            dir: dir.into(),
            stem: stem.into(),
            level,
            max_bytes: DEFAULT_MAX_BYTES,
            keep: DEFAULT_KEEP,
            append: false,
        }
    }

    /// A shared `<stem>.log` appended across runs and rotated only by size — for
    /// short-lived clients, where a fresh file per invocation would churn.
    pub fn appending(dir: impl Into<PathBuf>, stem: impl Into<String>, level: LogLevel) -> Self {
        FileLog {
            append: true,
            ..FileLog::new(dir, stem, level)
        }
    }

    /// The live log path, for callers that surface it (e.g. `daemon.status`).
    pub fn active_path(&self) -> PathBuf {
        if self.append {
            self.dir.join(format!("{}.log", self.stem))
        } else {
            self.dir.join(ACTIVE_NAME)
        }
    }

    fn open(self) -> io::Result<RollingLog> {
        if self.append {
            RollingLog::open_append(self.dir, &self.stem, self.max_bytes, self.keep)
        } else {
            RollingLog::open(self.dir, &self.stem, self.max_bytes, self.keep)
        }
    }
}

/// Holds the non-blocking file appender's worker; keep it alive for the lifetime
/// of the process, or buffered file logs are dropped at exit.
#[must_use = "dropping the guard stops file logging"]
pub struct LogGuard(#[allow(dead_code)] Option<tracing_appender::non_blocking::WorkerGuard>);

/// Configure the process-wide logger once at startup: the console (stderr) at
/// `console_level`, plus the optional file sink at its own level — the daemon's,
/// whose stderr is detached, and the CLI's, whose console must stay clean for
/// command output. A file-logging failure degrades to the console rather than
/// aborting startup.
pub fn init_logging(console_level: LogLevel, file: Option<FileLog>) -> LogGuard {
    let global = file
        .as_ref()
        .map(|f| f.level.filter())
        .unwrap_or(LevelFilter::OFF)
        .max(console_level.filter());
    let filter = EnvFilter::builder()
        .with_default_directive(global.into())
        .with_env_var("HESTIA_LOG")
        .from_env_lossy();

    let opened = file.and_then(|cfg| {
        let level = cfg.level.as_level()?;
        match cfg.open() {
            Ok(rolling) => Some((tracing_appender::non_blocking(rolling), level)),
            Err(e) => {
                eprintln!("hestia: file logging disabled: {e}");
                None
            }
        }
    });

    let (writer, ansi, guard) = match (opened, console_level.as_level()) {
        (Some(((file_writer, guard), file_level)), Some(console)) => (
            BoxMakeWriter::new(
                file_writer
                    .with_max_level(file_level)
                    .and(std::io::stderr.with_max_level(console)),
            ),
            false,
            Some(guard),
        ),
        (Some(((file_writer, guard), file_level)), None) => (
            BoxMakeWriter::new(file_writer.with_max_level(file_level)),
            false,
            Some(guard),
        ),
        (None, Some(console)) => (
            BoxMakeWriter::new(std::io::stderr.with_max_level(console)),
            true,
            None,
        ),
        (None, None) => (
            BoxMakeWriter::new(std::io::sink as fn() -> std::io::Sink),
            false,
            None,
        ),
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(cfg!(debug_assertions))
        .with_file(cfg!(debug_assertions))
        .with_writer(writer)
        .with_ansi(ansi)
        .init();

    LogGuard(guard)
}
