//! Process-wide logging configuration (tracing), the equivalent of the C++
//! `init_logging`. `LogLevel` is Hestia's own enum so callers don't depend on
//! tracing's types.

use std::path::Path;

use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::EnvFilter;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Off,
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

use tracing::level_filters::LevelFilter;

/// Holds the non-blocking file appender's worker; keep it alive for the lifetime
/// of the process, or buffered file logs are dropped at exit.
#[must_use = "dropping the guard stops file logging"]
pub struct LogGuard(#[allow(dead_code)] Option<tracing_appender::non_blocking::WorkerGuard>);

/// Configure the process-wide logger once at startup. When `file` is given, logs
/// also go to that path — used by the daemon, whose stderr is detached, so its
/// logs would otherwise be lost.
pub fn init_logging(level: LogLevel, file: Option<&Path>) -> LogGuard {
    let filter = EnvFilter::builder()
        .with_default_directive(level.filter().into())
        .with_env_var("HESTIA_LOG")
        .from_env_lossy();

    let stderr = std::io::stderr.with_max_level(tracing::Level::TRACE);

    if let Some(path) = file {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let (Some(dir), Some(name)) = (path.parent(), path.file_name()) {
            let appender = tracing_appender::rolling::never(dir, name);
            let (writer, guard) = tracing_appender::non_blocking(appender);
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(writer.and(stderr))
                .with_ansi(false)
                .init();
            return LogGuard(Some(guard));
        }
    }

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(stderr)
        .init();
    LogGuard(None)
}
