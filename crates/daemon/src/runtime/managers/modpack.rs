use std::sync::Arc;

use engine::Engine;
use proto::minecraft::ProvisionProgress;
use proto::modpack::{
    ModpackCancelledEvent, ModpackDoneEvent, ModpackErrorEvent, ModpackProgressEvent, ModpackRef,
    ModpackTarget,
};

use super::job::{coalesce_progress, job_id, topic_event, Cancellations, InFlight};
use crate::runtime::{instance_process_id, server_process_id, EventHub};

/// One pack install or update — what `ModpackManager::start` runs off-thread.
pub enum ModpackJob {
    InstallInstance {
        pack: ModpackRef,
        target: ModpackTarget,
    },
    InstallServer {
        pack: ModpackRef,
        target: ModpackTarget,
        eula: bool,
        port: Option<u16>,
    },
    UpdateInstance {
        instance_id: String,
        version: String,
        allow_downgrade: bool,
    },
    UpdateServer {
        server_id: String,
        version: String,
        allow_downgrade: bool,
    },
}

impl ModpackJob {
    /// The in-flight key: the entry's process id, shared with the content and
    /// backup jobs so nothing else touches the tree while a pack lands. An
    /// install that *creates* its entry has no key yet — the entry does not
    /// exist to conflict with — so it is keyed by its own job id instead.
    fn key(&self, id: &str) -> String {
        match self {
            ModpackJob::InstallInstance {
                target: ModpackTarget::Existing { entry },
                ..
            } => instance_process_id(entry),
            ModpackJob::InstallServer {
                target: ModpackTarget::Existing { entry },
                ..
            } => server_process_id(entry),
            ModpackJob::UpdateInstance { instance_id, .. } => instance_process_id(instance_id),
            ModpackJob::UpdateServer { server_id, .. } => server_process_id(server_id),
            _ => id.to_string(),
        }
    }

    fn id_prefix(&self) -> &'static str {
        match self {
            ModpackJob::InstallInstance { .. } => "instance-modpack-install",
            ModpackJob::InstallServer { .. } => "server-modpack-install",
            ModpackJob::UpdateInstance { .. } => "instance-modpack-update",
            ModpackJob::UpdateServer { .. } => "server-modpack-update",
        }
    }

    async fn run(
        self,
        engine: &Engine,
        on_progress: &engine::Job<'_>,
    ) -> anyhow::Result<engine::ModpackOutcome> {
        match self {
            ModpackJob::InstallInstance { pack, target } => {
                engine
                    .install_instance_modpack(&pack, &target, on_progress)
                    .await
            }
            ModpackJob::InstallServer {
                pack,
                target,
                eula,
                port,
            } => {
                engine
                    .install_server_modpack(&pack, &target, eula, port, on_progress)
                    .await
            }
            ModpackJob::UpdateInstance {
                instance_id,
                version,
                allow_downgrade,
            } => {
                engine
                    .update_instance_modpack(&instance_id, &version, allow_downgrade, on_progress)
                    .await
            }
            ModpackJob::UpdateServer {
                server_id,
                version,
                allow_downgrade,
            } => {
                engine
                    .update_server_modpack(&server_id, &version, allow_downgrade, on_progress)
                    .await
            }
        }
    }
}

pub struct ModpackManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    active: InFlight<String>,
    cancellations: Cancellations,
}

impl ModpackManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>, cancellations: Cancellations) -> Self {
        ModpackManager {
            engine,
            hub,
            active: InFlight::new(),
            cancellations,
        }
    }

    /// Whether a pack job is still running for this entry key
    /// (`server-<id>` / `instance-<id>`).
    pub fn in_flight(&self, key: &str) -> bool {
        self.active.contains(key)
    }

    /// Start a pack job off-thread, one per entry at a time. Returns the job id,
    /// or `None` if that entry is already busy.
    pub fn start(&self, job: ModpackJob, id: String) -> Option<String> {
        let id = job_id(id, job.id_prefix());
        let key = job.key(&id);
        let Some(claim) = self.active.claim(key.clone()) else {
            tracing::debug!(entry = %key, "modpack job already in flight");
            return None;
        };

        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let cancellations = self.cancellations.clone();
        let job_id = id.clone();
        tracing::info!(job = %id, entry = %key, kind = job.id_prefix(), "modpack job started");

        tokio::spawn(async move {
            let _claim = claim;
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress: Box<dyn Fn(&ProvisionProgress) + Send + Sync> =
                Box::new(coalesce_progress(move |p: &ProvisionProgress| {
                    progress_hub.publish(&topic_event(&ModpackProgressEvent {
                        id: progress_id.clone(),
                        progress: p.clone(),
                    }));
                }));

            let (cancel, _registered) = cancellations.register(&job_id);
            let running = engine::Job::new(on_progress.as_ref(), &cancel);
            match job.run(&engine, &running).await {
                Ok(outcome) => {
                    tracing::info!(
                        job = %job_id,
                        entry = %outcome.entry,
                        pack = %outcome.pack.name,
                        files = outcome.pack.files.len(),
                        failures = outcome.failures.len(),
                        "modpack job done"
                    );
                    hub.publish(&topic_event(&ModpackDoneEvent {
                        id: job_id.clone(),
                        entry: outcome.entry,
                        entry_name: outcome.entry_name,
                        pack: outcome.pack,
                        failures: outcome.failures,
                        warnings: outcome.warnings,
                    }));
                }
                Err(e) if engine::is_cancelled(&e) => {
                    tracing::info!(job = %job_id, "modpack job cancelled");
                    hub.publish(&topic_event(&ModpackCancelledEvent { id: job_id.clone() }));
                }
                Err(e) => {
                    tracing::error!(job = %job_id, error = format!("{e:#}"), "modpack job failed");
                    hub.publish(&topic_event(&ModpackErrorEvent {
                        id: job_id.clone(),
                        error: crate::runtime::engine_error(e),
                    }));
                }
            }
        });
        Some(id)
    }
}
