//! Instance import and export jobs.
//!
//! An export is keyed by the instance it reads, so it queues behind the same
//! in-flight claim a backup or content job takes — archiving a tree while
//! something else rewrites it would produce an archive of neither state. An
//! import has no entry to key on yet (that is what it creates), so it is keyed
//! by its own job id, exactly as a modpack install that creates its entry is.

use std::sync::Arc;

use engine::Engine;
use proto::minecraft::ProvisionProgress;
use proto::transfer::{
    ExportCancelledEvent, ExportDoneEvent, ExportErrorEvent, ExportFormat, ExportProgressEvent,
    ImportCancelledEvent, ImportDoneEvent, ImportErrorEvent, ImportProgressEvent,
};

use super::job::{job_id, topic_event, Cancellations, InFlight};
use crate::runtime::{instance_info, instance_process_id, EventHub};

/// One import or export — what `TransferManager::start` runs off-thread.
pub enum TransferJob {
    Export {
        instance_id: String,
        format: ExportFormat,
        destination: String,
        exclude: Vec<String>,
    },
    Import {
        path: String,
        name: String,
    },
}

impl TransferJob {
    fn key(&self, id: &str) -> String {
        match self {
            TransferJob::Export { instance_id, .. } => instance_process_id(instance_id),
            TransferJob::Import { .. } => id.to_string(),
        }
    }

    fn id_prefix(&self) -> &'static str {
        match self {
            TransferJob::Export { .. } => "instance-export",
            TransferJob::Import { .. } => "instance-import",
        }
    }
}

pub struct TransferManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    active: InFlight<String>,
    cancellations: Cancellations,
}

impl TransferManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>, cancellations: Cancellations) -> Self {
        TransferManager {
            engine,
            hub,
            active: InFlight::new(),
            cancellations,
        }
    }

    /// Whether a transfer is still running for this entry key
    /// (`instance-<id>`).
    pub fn in_flight(&self, key: &str) -> bool {
        self.active.contains(key)
    }

    /// Start a transfer off-thread. Returns the job id, or `None` when that
    /// entry is already busy.
    pub fn start(&self, job: TransferJob, id: String) -> Option<String> {
        let id = job_id(id, job.id_prefix());
        let key = job.key(&id);
        let Some(claim) = self.active.claim(key.clone()) else {
            tracing::debug!(entry = %key, "transfer job already in flight");
            return None;
        };

        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let cancellations = self.cancellations.clone();
        let job_id = id.clone();
        tracing::info!(job = %id, entry = %key, kind = job.id_prefix(), "transfer job started");

        tokio::spawn(async move {
            let _claim = claim;
            let (cancel, _registered) = cancellations.register(&job_id);
            match job {
                TransferJob::Export {
                    instance_id,
                    format,
                    destination,
                    exclude,
                } => {
                    let progress = export_progress(hub.clone(), job_id.clone());
                    let running = engine::Job::new(progress.as_ref(), &cancel);
                    let outcome = engine.export_instance(
                        &instance_id,
                        format,
                        &destination,
                        &exclude,
                        &running,
                    );
                    match outcome {
                        Ok(export) => {
                            tracing::info!(
                                job = %job_id,
                                files = export.files,
                                bytes = export.size_bytes,
                                "export done"
                            );
                            hub.publish(&topic_event(&ExportDoneEvent {
                                id: job_id.clone(),
                                path: export.path.to_string_lossy().into_owned(),
                                size_bytes: export.size_bytes,
                                files: export.files,
                                warnings: export.warnings,
                            }));
                        }
                        Err(e) if engine::is_cancelled(&e) => {
                            tracing::info!(job = %job_id, "export cancelled");
                            hub.publish(&topic_event(&ExportCancelledEvent { id: job_id.clone() }));
                        }
                        Err(e) => {
                            tracing::error!(job = %job_id, error = format!("{e:#}"), "export failed");
                            hub.publish(&topic_event(&ExportErrorEvent {
                                id: job_id.clone(),
                                error: crate::runtime::engine_error(e),
                            }));
                        }
                    }
                }
                TransferJob::Import { path, name } => {
                    let progress = import_progress(hub.clone(), job_id.clone());
                    let running = engine::Job::new(progress.as_ref(), &cancel);
                    match engine.import_instance(&path, &name, &running).await {
                        Ok(import) => {
                            tracing::info!(
                                job = %job_id,
                                instance = %import.record.id,
                                format = import.format.as_str(),
                                "import done"
                            );
                            // The view an import answers with is the same one
                            // `instance.list` returns, so a front-end can drop
                            // it straight into the library it already renders.
                            // A just-imported instance has no sessions by
                            // definition — nothing has launched it yet.
                            let accepts = engine.instance_accepts(&import.record.profile.flavor);
                            let instance = instance_info(import.record, Vec::new(), accepts);
                            hub.publish(&topic_event(&ImportDoneEvent {
                                id: job_id.clone(),
                                format: import.format,
                                instance,
                                failures: import.failures,
                                warnings: import.warnings,
                            }));
                        }
                        Err(e) if engine::is_cancelled(&e) => {
                            tracing::info!(job = %job_id, "import cancelled");
                            hub.publish(&topic_event(&ImportCancelledEvent { id: job_id.clone() }));
                        }
                        Err(e) => {
                            tracing::error!(job = %job_id, error = format!("{e:#}"), "import failed");
                            hub.publish(&topic_event(&ImportErrorEvent {
                                id: job_id.clone(),
                                error: crate::runtime::engine_error(e),
                            }));
                        }
                    }
                }
            }
        });
        Some(id)
    }
}

type Progress = Box<dyn Fn(&ProvisionProgress) + Send + Sync>;

fn export_progress(hub: Arc<EventHub>, id: String) -> Progress {
    Box::new(move |p| {
        hub.publish(&topic_event(&ExportProgressEvent {
            id: id.clone(),
            progress: p.clone(),
        }));
    })
}

fn import_progress(hub: Arc<EventHub>, id: String) -> Progress {
    Box::new(move |p| {
        hub.publish(&topic_event(&ImportProgressEvent {
            id: id.clone(),
            progress: p.clone(),
        }));
    })
}
