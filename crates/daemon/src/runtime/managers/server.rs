use std::sync::Arc;

use engine::{Engine, ServerCreateSpec, ServerUpdateSpec};
use proto::minecraft::ProvisionProgress;
use proto::server::{
    ServerCreateDoneEvent, ServerCreateErrorEvent, ServerCreateParams, ServerCreateProgressEvent,
    ServerUpdateDoneEvent, ServerUpdateErrorEvent, ServerUpdateParams, ServerUpdateProgressEvent,
};

use super::job::{coalesce_progress, job_id, topic_event, InFlight};
use crate::runtime::{server_info, EventHub};

pub struct ServerCreateManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    active: InFlight<String>,
}

impl ServerCreateManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>) -> Self {
        ServerCreateManager {
            engine,
            hub,
            active: InFlight::new(),
        }
    }

    /// Whether a create for this server name is still provisioning.
    pub fn in_flight(&self, name: &str) -> bool {
        self.active.contains(name)
    }

    /// Start a provisioning job off-thread, one per server name at a time.
    /// Returns the job id, or `None` if that name is already being created.
    pub fn start(&self, params: ServerCreateParams) -> Option<String> {
        let id = job_id(params.id.clone(), "server-create");
        let key = if params.name.trim().is_empty() {
            format!("{}-{}", params.flavor, params.version)
        } else {
            params.name.trim().to_string()
        };
        let Some(claim) = self.active.claim(key.clone()) else {
            tracing::debug!(server = %key, "server create already in flight");
            return None;
        };

        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let job_id = id.clone();
        tracing::info!(
            job = %id,
            name = %params.name,
            flavor = %params.flavor,
            version = %params.version,
            "server create started"
        );

        tokio::spawn(async move {
            let _claim = claim;
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress: Box<dyn Fn(&ProvisionProgress) + Send + Sync> =
                Box::new(coalesce_progress(move |p| {
                    progress_hub.publish(&topic_event(&ServerCreateProgressEvent {
                        id: progress_id.clone(),
                        progress: p.clone(),
                    }));
                }));

            let spec = ServerCreateSpec {
                name: params.name,
                flavor: params.flavor,
                version: params.version,
                loader_version: params.loader_version,
                port: params.port,
                config: params.config,
            };

            match engine.provision_server(spec, on_progress.as_ref()).await {
                Ok(record) => {
                    tracing::info!(
                        job = %job_id,
                        server = %record.id,
                        name = %record.name,
                        "server create done"
                    );
                    hub.publish(&topic_event(&ServerCreateDoneEvent {
                        id: job_id.clone(),
                        server: server_info(record, None),
                    }));
                }
                Err(e) => {
                    tracing::error!(job = %job_id, error = format!("{e:#}"), "server create failed");
                    hub.publish(&topic_event(&ServerCreateErrorEvent {
                        id: job_id.clone(),
                        message: format!("{e:#}"),
                    }));
                }
            }
        });
        Some(id)
    }
}

pub struct ServerUpdateManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    active: InFlight<String>,
}

impl ServerUpdateManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>) -> Self {
        ServerUpdateManager {
            engine,
            hub,
            active: InFlight::new(),
        }
    }

    /// Whether an update for this server id is still running.
    pub fn in_flight(&self, server_id: &str) -> bool {
        self.active.contains(server_id)
    }

    /// Start an update job off-thread, one per server at a time. Returns the
    /// job id, or `None` if that server is already being updated.
    pub fn start(&self, server_id: String, params: ServerUpdateParams) -> Option<String> {
        let id = job_id(params.id.clone(), "server-update");
        let Some(claim) = self.active.claim(server_id.clone()) else {
            tracing::debug!(server = %server_id, "server update already in flight");
            return None;
        };

        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let job_id = id.clone();
        tracing::info!(
            job = %id,
            server = %server_id,
            version = %params.version,
            allow_downgrade = params.allow_downgrade,
            "server update started"
        );

        tokio::spawn(async move {
            let _claim = claim;
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress: Box<dyn Fn(&ProvisionProgress) + Send + Sync> =
                Box::new(coalesce_progress(move |p| {
                    progress_hub.publish(&topic_event(&ServerUpdateProgressEvent {
                        id: progress_id.clone(),
                        progress: p.clone(),
                    }));
                }));

            let spec = ServerUpdateSpec {
                server: server_id.clone(),
                version: params.version,
                loader_version: params.loader_version,
                allow_downgrade: params.allow_downgrade,
            };

            match engine.update_server(spec, on_progress.as_ref()).await {
                Ok(record) => {
                    tracing::info!(
                        job = %job_id,
                        server = %record.id,
                        version = %record.profile.game_version,
                        "server update done"
                    );
                    hub.publish(&topic_event(&ServerUpdateDoneEvent {
                        id: job_id.clone(),
                        server: server_info(record, None),
                    }));
                }
                Err(e) => {
                    tracing::error!(job = %job_id, server = %server_id, error = format!("{e:#}"), "server update failed");
                    hub.publish(&topic_event(&ServerUpdateErrorEvent {
                        id: job_id.clone(),
                        message: format!("{e:#}"),
                    }));
                }
            }
        });
        Some(id)
    }
}
