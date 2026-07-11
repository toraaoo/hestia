//! Shared settings/configs: the set of game-relative settings files and folders
//! propagated across entries through the launcher's `shared/` store. The store is
//! copy-based (each entry keeps its own physical copy under `data/`), so nothing
//! is live-shared and backups stay intact — see the engine's `sync` subsystem.
//!
//! Targets — and the store itself — are **kept separate per entry kind**: a
//! server and an instance sync different files (a server has no `options.txt`),
//! and a server's mod `config/` must not mix with a client's, so each kind has
//! its own target list and its own `shared/<kind>/` store.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty};

/// Which entry kind a target set belongs to.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SyncKind {
    Server,
    Instance,
}

/// The game-relative paths shared across entries of one kind: individual `files`
/// (copied newest-wins, `options.txt` key-merged) and whole `folders` (every file
/// under them synced newest-wins per relative path).
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct SyncTargets {
    pub files: BTreeSet<String>,
    pub folders: BTreeSet<String>,
}

/// The sync store location plus each kind's current targets.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct SyncConfig {
    pub shared_dir: PathBuf,
    pub servers: SyncTargets,
    pub instances: SyncTargets,
}

pub struct SyncGet;
impl Contract for SyncGet {
    const CHANNEL: &'static str = "sync.get";
    type Params = Empty;
    type Result = SyncConfig;
}

/// Replace one kind's target set wholesale. The daemon validates each path
/// (relative, no `..` escape, not a launcher-managed directory) before persisting.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SyncSetParams {
    pub kind: SyncKind,
    #[serde(default)]
    pub targets: SyncTargets,
}

pub struct SyncSet;
impl Contract for SyncSet {
    const CHANNEL: &'static str = "sync.set";
    type Params = SyncSetParams;
    type Result = SyncConfig;
}
