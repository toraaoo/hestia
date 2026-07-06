//! Process-wide logging configuration (tracing). `LogLevel` is Hestia's own enum
//! so callers don't depend on tracing's types. The daemon also writes to a
//! rotating, compressed file (see [`FileLog`]); short-lived clients log to stderr.

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
}

const DEFAULT_MAX_BYTES: u64 = 10 * 1024 * 1024;
const DEFAULT_KEEP: usize = 8;

/// A rotating file log: the live `latest.log` under `dir`, rotated to a dated
/// gzip archive (`<stem>-YYYY-MM-DD-N.log.gz`) on startup and once it exceeds
/// `max_bytes`, keeping the newest `keep` archives.
pub struct FileLog {
    dir: PathBuf,
    stem: String,
    max_bytes: u64,
    keep: usize,
}

impl FileLog {
    pub fn new(dir: impl Into<PathBuf>, stem: impl Into<String>) -> Self {
        FileLog {
            dir: dir.into(),
            stem: stem.into(),
            max_bytes: DEFAULT_MAX_BYTES,
            keep: DEFAULT_KEEP,
        }
    }

    /// The live log path, for callers that surface it (e.g. `daemon.status`).
    pub fn active_path(&self) -> PathBuf {
        self.dir.join(ACTIVE_NAME)
    }
}

/// Holds the non-blocking file appender's worker; keep it alive for the lifetime
/// of the process, or buffered file logs are dropped at exit.
#[must_use = "dropping the guard stops file logging"]
pub struct LogGuard(#[allow(dead_code)] Option<tracing_appender::non_blocking::WorkerGuard>);

/// Configure the process-wide logger once at startup. When `file` is given, logs
/// also go to a rotating, compressed file there — used by the daemon, whose stderr
/// is detached, so its logs would otherwise be lost. A file-logging failure
/// degrades to stderr rather than aborting startup.
pub fn init_logging(level: LogLevel, file: Option<FileLog>) -> LogGuard {
    let filter = EnvFilter::builder()
        .with_default_directive(level.filter().into())
        .with_env_var("HESTIA_LOG")
        .from_env_lossy();

    let stderr = std::io::stderr.with_max_level(tracing::Level::TRACE);

    let non_blocking =
        file.and_then(
            |cfg| match RollingLog::open(cfg.dir, &cfg.stem, cfg.max_bytes, cfg.keep) {
                Ok(rolling) => Some(tracing_appender::non_blocking(rolling)),
                Err(e) => {
                    eprintln!("hestia: file logging disabled: {e}");
                    None
                }
            },
        );

    let (writer, ansi, guard) = match non_blocking {
        Some((file_writer, guard)) => (
            BoxMakeWriter::new(file_writer.and(stderr)),
            false,
            Some(guard),
        ),
        None => (BoxMakeWriter::new(stderr), true, None),
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
