//! Shared settings/configs: the set of game-relative settings files and folders
//! propagated across instances through the launcher's `shared/` store. The store
//! is copy-based (each instance keeps its own physical copy under `data/`), so
//! nothing is live-shared and backups stay intact — see the engine's `sync`
//! subsystem.
//!
//! Sync is **instance-only**: it is a client-side quality-of-life feature. A
//! server's configuration is per-server infrastructure, managed through its own
//! `server.config.*` keys and `server.properties`, never a cross-entry store.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty};

/// The game-relative paths shared across instances: individual `files`
/// (copied newest-wins, `options.txt` key-merged) and whole `folders` (every file
/// under them synced newest-wins per relative path).
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct SyncTargets {
    pub files: BTreeSet<String>,
    pub folders: BTreeSet<String>,
}

/// The sync store location plus the current targets.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct SyncConfig {
    pub shared_dir: PathBuf,
    pub targets: SyncTargets,
}

pub struct SyncGet;
impl Contract for SyncGet {
    const CHANNEL: &'static str = "sync.get";
    type Params = Empty;
    type Result = SyncConfig;
}

/// Replace the target set wholesale. The daemon validates each path
/// (relative, no `..` escape, not a launcher-managed directory) before persisting.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct SyncSetParams {
    pub targets: SyncTargets,
}

pub struct SyncSet;
impl Contract for SyncSet {
    const CHANNEL: &'static str = "sync.set";
    type Params = SyncSetParams;
    type Result = SyncConfig;
}
