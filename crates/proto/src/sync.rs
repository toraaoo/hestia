//! Shared settings/configs: the set of game-relative settings targets
//! propagated across instances through the launcher's `shared/` store. `files`
//! are copied (newest-wins, `options.txt` key-merged); `folders` are **linked**
//! (a symlink on POSIX, a junction on Windows) into the store, so folder
//! content — worlds above all — is stored once and shared live. See the
//! engine's `sync` subsystem.
//!
//! Sync is **instance-only**: it is a client-side quality-of-life feature. A
//! server's configuration is per-server infrastructure, managed through its own
//! `server.config.*` keys and `server.properties`, never a cross-entry store.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty};

/// The game-relative paths shared across instances: individual `files`
/// (copied newest-wins, `options.txt` key-merged) and whole `folders` (linked
/// into the shared store — every instance opens the same physical directory).
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

/// One folder target's link state on one instance.
#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LinkState {
    /// Linked into the shared store.
    Linked,
    /// Missing, empty, or a stale hestia-store link — the next launch links it.
    #[default]
    Pending,
    /// A non-empty real directory (or a foreign link) that will never be
    /// linked automatically; `instance <name> sync adopt` is the fix.
    CannotLink,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct TargetLinkState {
    pub target: String,
    pub state: LinkState,
}

/// One instance's per-folder-target link states.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceSyncStatus {
    pub id: String,
    pub name: String,
    pub targets: Vec<TargetLinkState>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct SyncStatusResult {
    pub instances: Vec<InstanceSyncStatus>,
}

pub struct SyncStatus;
impl Contract for SyncStatus {
    const CHANNEL: &'static str = "sync.status";
    type Params = Empty;
    type Result = SyncStatusResult;
}

/// Adopt an instance's existing folder contents into the shared store: move
/// the entries under each named target (all folder targets when empty) and
/// link the emptied folder. All-or-nothing per target — a name collision with
/// the store refuses that whole target.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct SyncAdoptParams {
    /// Instance name or id.
    pub instance: String,
    /// Folder targets to adopt; empty adopts every folder target.
    pub targets: Vec<String>,
}

/// What adopt did per target.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct SyncAdoptResult {
    /// Targets now linked (adopted by this call or already linked).
    pub adopted: Vec<String>,
}

pub struct SyncAdopt;
impl Contract for SyncAdopt {
    const CHANNEL: &'static str = "instance.sync.adopt";
    type Params = SyncAdoptParams;
    type Result = SyncAdoptResult;
}
