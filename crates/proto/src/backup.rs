//! Server backup contracts: archive a server's `data/` directory on demand,
//! list/restore/remove the stored archives. Create and restore are jobs — the
//! call answers with the job id and progress/done/error arrive as events, like
//! the other provisioning jobs. Servers additionally back up on a schedule and
//! before a version update — the schedule is configured through the server's
//! `config` channels (`backup-interval` / `backup-retention`), not a channel
//! of its own. Backups are a **server** feature: instances have none
//! (import/export is the intended replacement, not yet built).

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty, Topic};
use crate::minecraft::ProvisionProgress;
use crate::server::ServerRef;

/// Why a backup was taken. Encoded into the archive's id, so the disk stays
/// the registry.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackupKind {
    #[default]
    Manual,
    Scheduled,
    /// Taken automatically before a version update.
    Update,
}

impl BackupKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackupKind::Manual => "manual",
            BackupKind::Scheduled => "scheduled",
            BackupKind::Update => "update",
        }
    }

    pub fn parse(value: &str) -> Option<BackupKind> {
        match value {
            "manual" => Some(BackupKind::Manual),
            "scheduled" => Some(BackupKind::Scheduled),
            "update" => Some(BackupKind::Update),
            _ => None,
        }
    }
}

/// One stored backup archive.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct BackupInfo {
    pub id: String,
    pub kind: BackupKind,
    pub created_unix: i64,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct BackupListResult {
    pub backups: Vec<BackupInfo>,
}

/// The immediate answer of a backup create/restore call: the job whose
/// events carry the outcome.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct BackupJobResult {
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerBackupCreateParams {
    /// Server name or id.
    pub server: String,
    /// Client-supplied job id; empty asks the daemon to allocate one.
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerBackupRestoreParams {
    /// Server name or id.
    pub server: String,
    /// Backup id (`server.backup.list` names them).
    pub backup: String,
    /// Client-supplied job id; empty asks the daemon to allocate one.
    pub id: String,
}

/// Names one backup of one server (remove).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerBackupRef {
    pub server: String,
    pub backup: String,
}

pub struct ServerBackupCreate;
impl Contract for ServerBackupCreate {
    const CHANNEL: &'static str = "server.backup.create";
    type Params = ServerBackupCreateParams;
    type Result = BackupJobResult;
}

pub struct ServerBackupList;
impl Contract for ServerBackupList {
    const CHANNEL: &'static str = "server.backup.list";
    type Params = ServerRef;
    type Result = BackupListResult;
}

pub struct ServerBackupRestore;
impl Contract for ServerBackupRestore {
    const CHANNEL: &'static str = "server.backup.restore";
    type Params = ServerBackupRestoreParams;
    type Result = BackupJobResult;
}

pub struct ServerBackupRemove;
impl Contract for ServerBackupRemove {
    const CHANNEL: &'static str = "server.backup.remove";
    type Params = ServerBackupRef;
    type Result = Empty;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupProgressEvent {
    pub id: String,
    #[serde(flatten)]
    pub progress: ProvisionProgress,
}
impl Topic for BackupProgressEvent {
    const TOPIC: &'static str = "backup.progress";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupDoneEvent {
    pub id: String,
    pub backup: BackupInfo,
}
impl Topic for BackupDoneEvent {
    const TOPIC: &'static str = "backup.done";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupErrorEvent {
    pub id: String,
    pub message: String,
}
impl Topic for BackupErrorEvent {
    const TOPIC: &'static str = "backup.error";
}
