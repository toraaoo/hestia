use std::sync::Arc;

use engine::Engine;
use proto::content::{
    ContentAddSpec, ContentDoneEvent, ContentErrorEvent, ContentFailure, ContentKind,
    ContentProgressEvent, InstalledContent,
};
use proto::minecraft::ProvisionProgress;

use super::job::{job_id, topic_event, InFlight};
use crate::runtime::{instance_process_id, server_process_id, EventHub};

/// One content install or update for one entry — what `ContentManager::start`
/// runs off-thread.
pub enum ContentJob {
    ServerAdd {
        server_id: String,
        spec: ContentAddSpec,
    },
    InstanceAdd {
        instance_id: String,
        spec: ContentAddSpec,
    },
    ServerUpdate {
        server_id: String,
        kind: ContentKind,
        item: String,
    },
    InstanceUpdate {
        instance_id: String,
        kind: ContentKind,
        item: String,
    },
}

impl ContentJob {
    /// The in-flight key: one content change per entry at a time, keyed by the
    /// entry's process id like the backup jobs.
    fn key(&self) -> String {
        match self {
            ContentJob::ServerAdd { server_id, .. }
            | ContentJob::ServerUpdate { server_id, .. } => server_process_id(server_id),
            ContentJob::InstanceAdd { instance_id, .. }
            | ContentJob::InstanceUpdate { instance_id, .. } => instance_process_id(instance_id),
        }
    }

    fn id_prefix(&self) -> &'static str {
        match self {
            ContentJob::ServerAdd { .. } => "server-content-add",
            ContentJob::InstanceAdd { .. } => "instance-content-add",
            ContentJob::ServerUpdate { .. } => "server-content-update",
            ContentJob::InstanceUpdate { .. } => "instance-content-update",
        }
    }

    async fn run(
        self,
        engine: &Engine,
        on_progress: &(dyn Fn(&ProvisionProgress) + Send + Sync),
    ) -> anyhow::Result<(Vec<InstalledContent>, Vec<ContentFailure>)> {
        match self {
            ContentJob::ServerAdd { server_id, spec } => {
                engine
                    .add_server_content(&server_id, &spec, on_progress)
                    .await
            }
            ContentJob::InstanceAdd { instance_id, spec } => {
                engine
                    .add_instance_content(&instance_id, &spec, on_progress)
                    .await
            }
            ContentJob::ServerUpdate {
                server_id,
                kind,
                item,
            } => engine
                .update_server_content(&server_id, kind, &item, on_progress)
                .await
                .map(|items| (items, Vec::new())),
            ContentJob::InstanceUpdate {
                instance_id,
                kind,
                item,
            } => engine
                .update_instance_content(&instance_id, kind, &item, on_progress)
                .await
                .map(|items| (items, Vec::new())),
        }
    }
}

pub struct ContentManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    active: InFlight<String>,
}

impl ContentManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>) -> Self {
        ContentManager {
            engine,
            hub,
            active: InFlight::new(),
        }
    }

    /// Whether a content change is still running for this entry key
    /// (`server-<id>` / `instance-<id>`).
    pub fn in_flight(&self, key: &str) -> bool {
        self.active.contains(key)
    }

    /// Start an install/update job off-thread, one per entry at a time.
    /// Returns the job id, or `None` if that entry is already busy.
    pub fn start(&self, job: ContentJob, id: String) -> Option<String> {
        let id = job_id(id, job.id_prefix());
        let key = job.key();
        let Some(claim) = self.active.claim(key.clone()) else {
            tracing::debug!(entry = %key, "content job already in flight");
            return None;
        };

        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let job_id = id.clone();
        tracing::info!(job = %id, entry = %key, kind = job.id_prefix(), "content job started");

        tokio::spawn(async move {
            let _claim = claim;
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress: Box<dyn Fn(&ProvisionProgress) + Send + Sync> = Box::new(move |p| {
                progress_hub.publish(&topic_event(&ContentProgressEvent {
                    id: progress_id.clone(),
                    progress: p.clone(),
                }));
            });

            match job.run(&engine, on_progress.as_ref()).await {
                Ok((items, failures)) => {
                    tracing::info!(
                        job = %job_id,
                        items = items.len(),
                        failures = failures.len(),
                        "content job done"
                    );
                    hub.publish(&topic_event(&ContentDoneEvent {
                        id: job_id.clone(),
                        items,
                        failures,
                    }));
                }
                Err(e) => {
                    tracing::error!(job = %job_id, error = format!("{e:#}"), "content job failed");
                    hub.publish(&topic_event(&ContentErrorEvent {
                        id: job_id.clone(),
                        message: format!("{e:#}"),
                    }));
                }
            }
        });
        Some(id)
    }
}
