//! Cross-cutting: logging, app identity, path resolution. Linked by the daemon
//! and every client; zero UI or domain dependencies.

pub mod app;
pub mod logging;
pub mod paths;

pub use logging::{init_logging, LogGuard, LogLevel};
