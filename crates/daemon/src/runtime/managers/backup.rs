use std::sync::Arc;

use engine::Engine;
use proto::backup::{
    BackupDoneEvent, BackupErrorEvent, BackupInfo, BackupKind, BackupProgressEvent,
};
use proto::minecraft::ProvisionProgress;

use super::job::{job_id, topic_event, InFlight};
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

    async fn run(
        self,
        engine: &Engine,
        on_progress: &(dyn Fn(&ProvisionProgress) + Send + Sync),
    ) -> anyhow::Result<BackupInfo> {
        match self {
            BackupJob::ServerBackup { server_id, live } => {
                engine
                    .backup_server(&server_id, BackupKind::Manual, live, on_progress)
                    .await
            }
            BackupJob::ServerRestore { server_id, backup } => {
                engine
                    .restore_server_backup(&server_id, &backup, on_progress)
                    .await
            }
        }
    }
}

pub struct BackupManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    active: InFlight<String>,
}

impl BackupManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>) -> Self {
        BackupManager {
            engine,
            hub,
            active: InFlight::new(),
        }
    }

    /// Whether a backup or restore is still running for this server key
    /// (`server-<id>`).
    pub fn in_flight(&self, key: &str) -> bool {
        self.active.contains(key)
    }

    /// Start a backup/restore job off-thread, one per entry at a time.
    /// Returns the job id, or `None` if that entry is already busy.
    pub fn start(&self, job: BackupJob, id: String) -> Option<String> {
        let id = job_id(id, job.id_prefix());
        let key = job.key();
        let Some(claim) = self.active.claim(key.clone()) else {
            tracing::debug!(entry = %key, "backup job already in flight");
            return None;
        };

        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let job_id = id.clone();
        tracing::info!(job = %id, entry = %key, kind = job.id_prefix(), "backup job started");

        tokio::spawn(async move {
            let _claim = claim;
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress: Box<dyn Fn(&ProvisionProgress) + Send + Sync> = Box::new(move |p| {
                progress_hub.publish(&topic_event(&BackupProgressEvent {
                    id: progress_id.clone(),
                    progress: p.clone(),
                }));
            });

            match job.run(&engine, on_progress.as_ref()).await {
                Ok(backup) => {
                    tracing::info!(job = %job_id, backup = %backup.id, size = backup.size, "backup job done");
                    hub.publish(&topic_event(&BackupDoneEvent {
                        id: job_id.clone(),
                        backup,
                    }));
                }
                Err(e) => {
                    tracing::error!(job = %job_id, error = format!("{e:#}"), "backup job failed");
                    hub.publish(&topic_event(&BackupErrorEvent {
                        id: job_id.clone(),
                        message: format!("{e:#}"),
                    }));
                }
            }
        });
        Some(id)
    }
}
