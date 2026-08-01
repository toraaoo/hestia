use std::sync::Arc;

use engine::Engine;
use proto::content::{
    ContentAddSpec, ContentCancelledEvent, ContentDoneEvent, ContentErrorEvent, ContentFailure,
    ContentKind, ContentProgressEvent, InstalledContent,
};
use proto::minecraft::ProvisionProgress;

use super::job::{progress_event, settle, Cancellations, Runner, Spec};
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
    /// Re-pin one item to a specific published version.
    ServerSetVersion {
        server_id: String,
        kind: ContentKind,
        item: String,
        version: String,
    },
    InstanceSetVersion {
        instance_id: String,
        kind: ContentKind,
        item: String,
        version: String,
    },
    /// Apply a global profile's references into an instance's pool.
    ProfileApply {
        instance_id: String,
        profile: String,
    },
}

type Installed = (Vec<InstalledContent>, Vec<ContentFailure>);

impl ContentJob {
    /// The in-flight key: one content change per entry at a time, keyed by the
    /// entry's process id like the backup jobs.
    fn key(&self) -> String {
        match self {
            ContentJob::ServerAdd { server_id, .. }
            | ContentJob::ServerUpdate { server_id, .. }
            | ContentJob::ServerSetVersion { server_id, .. } => server_process_id(server_id),
            ContentJob::InstanceAdd { instance_id, .. }
            | ContentJob::InstanceUpdate { instance_id, .. }
            | ContentJob::InstanceSetVersion { instance_id, .. }
            | ContentJob::ProfileApply { instance_id, .. } => instance_process_id(instance_id),
        }
    }

    fn id_prefix(&self) -> &'static str {
        match self {
            ContentJob::ServerAdd { .. } => "server-content-add",
            ContentJob::InstanceAdd { .. } => "instance-content-add",
            ContentJob::ServerUpdate { .. } => "server-content-update",
            ContentJob::InstanceUpdate { .. } => "instance-content-update",
            ContentJob::ServerSetVersion { .. } => "server-content-set-version",
            ContentJob::InstanceSetVersion { .. } => "instance-content-set-version",
            ContentJob::ProfileApply { .. } => "profile-apply",
        }
    }

    async fn run(
        self,
        engine: &Engine,
        on_progress: &engine::Job<'_>,
    ) -> anyhow::Result<Installed> {
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
            ContentJob::ServerSetVersion {
                server_id,
                kind,
                item,
                version,
            } => engine
                .set_server_content_version(&server_id, kind, &item, &version, on_progress)
                .await
                .map(|items| (items, Vec::new())),
            ContentJob::InstanceSetVersion {
                instance_id,
                kind,
                item,
                version,
            } => engine
                .set_instance_content_version(&instance_id, kind, &item, &version, on_progress)
                .await
                .map(|items| (items, Vec::new())),
            ContentJob::ProfileApply {
                instance_id,
                profile,
            } => {
                engine
                    .apply_global_profile(&instance_id, &profile, on_progress)
                    .await
            }
        }
    }
}

pub struct ContentManager {
    runner: Runner<String>,
}

impl ContentManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>, cancellations: Cancellations) -> Self {
        ContentManager {
            runner: Runner::new(engine, hub, cancellations),
        }
    }

    /// Whether a content change is still running for this entry key
    /// (`server-<id>` / `instance-<id>`).
    pub fn in_flight(&self, key: &str) -> bool {
        self.runner.in_flight(key)
    }

    /// Start an install/update job off-thread, one per entry at a time.
    /// Returns the job id, or `None` if that entry is already busy.
    pub fn start(&self, job: ContentJob, id: String) -> Option<String> {
        let spec = Spec {
            id,
            prefix: job.id_prefix(),
            key: Some(job.key()),
            progress: progress_event(|id, progress: &ProvisionProgress| ContentProgressEvent {
                id,
                progress: progress.clone(),
            }),
            done: settle(|id, (items, failures): Installed| ContentDoneEvent {
                id,
                items,
                failures,
            }),
            cancelled: Some(settle(|id, ()| ContentCancelledEvent { id })),
            error: settle(|id, e| ContentErrorEvent {
                id,
                error: crate::runtime::engine_error(e),
            }),
        };

        self.runner.start(spec, move |engine, reporter| {
            Box::pin(async move { job.run(engine, &reporter.job()).await })
        })
    }
}
