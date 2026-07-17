//! The launcher engine: config, cache, downloader, java, accounts, minecraft
//! providers, and the server/instance stores with their launch pipeline.
//! Daemon-only domain logic; front-ends reach it over the socket, not by
//! linking it.

mod accounts;
mod backup;
mod cache;
mod checksum;
mod config;
mod content;
mod download;
mod engine;
mod instances;
mod java;
mod minecraft;
mod profiles;
mod registry;
mod servers;
mod skins;
mod sync;

pub use accounts::{Accounts, LoginChallenge};
pub use backup::BackupSettings;
pub use cache::{Cache, CacheEntry, CacheUsage};
pub use config::{Config, ConfigError, Settings};
pub use content::Content;
pub use download::Downloader;
pub use engine::{Engine, ServerCreateSpec, ServerUpdateSpec};
pub use instances::{InstanceRecord, Instances};
pub use java::{Java, JavaInstallOutcome};
pub use minecraft::launch::{JavaSettings, LaunchPlan};
pub use minecraft::Minecraft;
pub use profiles::Profiles;
pub use servers::{RconConfig, ServerRecord, Servers};
pub use skins::Skins;
pub use sync::Sync;
