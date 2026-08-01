use std::sync::Arc;

use engine::Engine;
use proto::backup::{
    BackupCancelledEvent, BackupDoneEvent, BackupErrorEvent, BackupInfo, BackupKind,
    BackupProgressEvent,
};
use proto::minecraft::ProvisionProgress;

use super::job::{progress_event, settle, Cancellations, Runner, Spec};
use crate::runtime::{server_process_id, EventHub};

/// One backup or restore job for one server — what `BackupManager::start`
/// runs off-thread. Backups are a server feature; instances have none.
pub enum BackupJob {
    ServerBackup { server_id: String, live: bool },
    ServerRestore { server_id: String, backup: String },
}

impl BackupJob {
    /// The in-flight key: one backup *or* restore per server at a time. The
    /// server's process id is the key, so handlers can check it without
    /// re-deriving a format.
    fn key(&self) -> String {
        match self {
            BackupJob::ServerBackup { server_id, .. }
            | BackupJob::ServerRestore { server_id, .. } => server_process_id(server_id),
        }
    }

    fn id_prefix(&self) -> &'static str {
        match self {
            BackupJob::ServerBackup { .. } => "server-backup",
            BackupJob::ServerRestore { .. } => "server-restore",
        }
    }
}

pub struct BackupManager {
    runner: Runner<String>,
}

impl BackupManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>, cancellations: Cancellations) -> Self {
        BackupManager {
            runner: Runner::new(engine, hub, cancellations),
        }
    }

    /// Whether a backup or restore is still running for this server key
    /// (`server-<id>`).
    pub fn in_flight(&self, key: &str) -> bool {
        self.runner.in_flight(key)
    }

    /// Start a backup/restore job off-thread, one per entry at a time.
    /// Returns the job id, or `None` if that entry is already busy.
    pub fn start(&self, job: BackupJob, id: String) -> Option<String> {
        let spec = Spec {
            id,
            prefix: job.id_prefix(),
            key: Some(job.key()),
            progress: progress_event(|id, progress: &ProvisionProgress| BackupProgressEvent {
                id,
                progress: progress.clone(),
            }),
            done: settle(|id, backup: BackupInfo| BackupDoneEvent { id, backup }),
            cancelled: Some(settle(|id, ()| BackupCancelledEvent { id })),
            error: settle(|id, e| BackupErrorEvent {
                id,
                error: crate::runtime::engine_error(e),
            }),
        };

        self.runner.start(spec, move |engine, reporter| {
            Box::pin(async move {
                let running = reporter.job();
                match job {
                    BackupJob::ServerBackup { server_id, live } => {
                        engine
                            .backup_server(&server_id, BackupKind::Manual, live, &running)
                            .await
                    }
                    BackupJob::ServerRestore { server_id, backup } => {
                        engine
                            .restore_server_backup(&server_id, &backup, &running)
                            .await
                    }
                }
            })
        })
    }
}
