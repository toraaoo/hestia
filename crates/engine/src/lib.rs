//! The launcher engine: config, cache, downloader, java. Daemon-only domain
//! logic; front-ends reach it over the socket, not by linking it. Accounts +
//! crypto are not implemented yet.

mod cache;
mod checksum;
mod config;
mod download;
mod engine;
mod java;

pub use cache::{Cache, CacheEntry, CacheUsage};
pub use config::{Config, ConfigError, Settings};
pub use download::Downloader;
pub use engine::Engine;
pub use java::{Java, JavaInstallOutcome};
