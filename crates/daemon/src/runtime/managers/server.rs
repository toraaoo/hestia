use std::sync::Arc;

use engine::{Engine, ServerCreateSpec, ServerUpdateSpec};
use proto::minecraft::ProvisionProgress;
use proto::server::{
    ServerCreateCancelledEvent, ServerCreateDoneEvent, ServerCreateErrorEvent, ServerCreateParams,
    ServerCreateProgressEvent, ServerInfo, ServerUpdateCancelledEvent, ServerUpdateDoneEvent,
    ServerUpdateErrorEvent, ServerUpdateParams, ServerUpdateProgressEvent,
};
use proto::warning::WarningInfo;

use super::job::{progress_event, settle, Cancellations, Runner, Spec};
use crate::runtime::{server_info, EventHub};

/// What both server jobs settle with: the wire's view of the server, plus
/// whatever the provision degraded on. Assembled where the engine is in reach,
/// so the done event stays a plain constructor.
type Provisioned = (ServerInfo, Vec<WarningInfo>);

fn provisioned(engine: &Engine, outcome: (engine::ServerRecord, Vec<WarningInfo>)) -> Provisioned {
    let (record, warnings) = outcome;
    let accepts = engine.server_accepts(&record.profile.flavor);
    (server_info(record, None, accepts), warnings)
}

pub struct ServerCreateManager {
    runner: Runner<String>,
}

impl ServerCreateManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>, cancellations: Cancellations) -> Self {
        ServerCreateManager {
            runner: Runner::new(engine, hub, cancellations),
        }
    }

    /// Whether a create for this server name is still provisioning.
    pub fn in_flight(&self, name: &str) -> bool {
        self.runner.in_flight(name)
    }

    /// Start a provisioning job off-thread, one per server name at a time.
    /// Returns the job id, or `None` if that name is already being created.
    pub fn start(&self, params: ServerCreateParams) -> Option<String> {
        let key = if params.name.trim().is_empty() {
            format!("{}-{}", params.flavor, params.version)
        } else {
            params.name.trim().to_string()
        };
        let spec = Spec {
            id: params.id.clone(),
            prefix: "server-create",
            key: Some(key),
            progress: progress_event(|id, progress: &ProvisionProgress| {
                ServerCreateProgressEvent {
                    id,
                    progress: progress.clone(),
                }
            }),
            done: settle(
                |id, (server, warnings): Provisioned| ServerCreateDoneEvent {
                    id,
                    server,
                    warnings,
                },
            ),
            cancelled: Some(settle(|id, ()| ServerCreateCancelledEvent { id })),
            error: settle(|id, e| ServerCreateErrorEvent {
                id,
                error: crate::runtime::engine_error(e),
            }),
        };

        self.runner.start(spec, move |engine, reporter| {
            Box::pin(async move {
                let create = ServerCreateSpec {
                    name: params.name,
                    flavor: params.flavor,
                    version: params.version,
                    loader_version: params.loader_version,
                    port: params.port,
                    config: params.config,
                    eula: params.eula,
                };
                let outcome = engine.provision_server(create, &reporter.job()).await?;
                Ok(provisioned(engine, outcome))
            })
        })
    }
}

pub struct ServerUpdateManager {
    runner: Runner<String>,
}

impl ServerUpdateManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>, cancellations: Cancellations) -> Self {
        ServerUpdateManager {
            runner: Runner::new(engine, hub, cancellations),
        }
    }

    /// Whether an update for this server id is still running.
    pub fn in_flight(&self, server_id: &str) -> bool {
        self.runner.in_flight(server_id)
    }

    /// Start an update job off-thread, one per server at a time. Returns the
    /// job id, or `None` if that server is already being updated.
    pub fn start(&self, server_id: String, params: ServerUpdateParams) -> Option<String> {
        let spec = Spec {
            id: params.id.clone(),
            prefix: "server-update",
            key: Some(server_id.clone()),
            progress: progress_event(|id, progress: &ProvisionProgress| {
                ServerUpdateProgressEvent {
                    id,
                    progress: progress.clone(),
                }
            }),
            done: settle(
                |id, (server, warnings): Provisioned| ServerUpdateDoneEvent {
                    id,
                    server,
                    warnings,
                },
            ),
            cancelled: Some(settle(|id, ()| ServerUpdateCancelledEvent { id })),
            error: settle(|id, e| ServerUpdateErrorEvent {
                id,
                error: crate::runtime::engine_error(e),
            }),
        };

        self.runner.start(spec, move |engine, reporter| {
            Box::pin(async move {
                let update = ServerUpdateSpec {
                    server: server_id,
                    version: params.version,
                    loader_version: params.loader_version,
                    allow_downgrade: params.allow_downgrade,
                };
                let outcome = engine.update_server(update, &reporter.job()).await?;
                Ok(provisioned(engine, outcome))
            })
        })
    }
}
