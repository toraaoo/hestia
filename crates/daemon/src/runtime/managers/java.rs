use std::sync::Arc;

use engine::Engine;
use proto::java::{JavaInstallDoneEvent, JavaInstallErrorEvent};

use super::job::{job_id, topic_event, InFlight};
use crate::runtime::EventHub;

pub struct JavaInstallManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    active: InFlight<i32>,
}

impl JavaInstallManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>) -> Self {
        JavaInstallManager {
            engine,
            hub,
            active: InFlight::new(),
        }
    }

    /// Start an install off-thread, one per release line at a time. Returns the
    /// job id, or `None` if that line is already installing.
    pub fn start(&self, major: i32, id: String, force: bool) -> Option<String> {
        let id = job_id(id, "java-install");
        let Some(claim) = self.active.claim(major) else {
            tracing::debug!(major, "java install already in flight");
            return None;
        };

        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let job_id = id.clone();
        tracing::info!(job = %id, major, force, "java install started");

        tokio::spawn(async move {
            let _claim = claim;
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress = move |p: &proto::java::JavaInstallProgress| {
                progress_hub.publish(&topic_event(&proto::java::JavaInstallProgressEvent {
                    id: progress_id.clone(),
                    progress: p.clone(),
                }));
            };

            let result = engine
                .java()
                .install(major, force, Some(engine.cache()), on_progress)
                .await;

            match result {
                Ok(outcome) => {
                    tracing::info!(
                        job = %job_id,
                        major,
                        already_installed = outcome.already_installed,
                        "java install done"
                    );
                    hub.publish(&topic_event(&JavaInstallDoneEvent {
                        id: job_id.clone(),
                        runtime: outcome.runtime,
                        already_installed: outcome.already_installed,
                    }));
                }
                Err(e) => {
                    tracing::error!(job = %job_id, major, error = format!("{e:#}"), "java install failed");
                    hub.publish(&topic_event(&JavaInstallErrorEvent {
                        id: job_id.clone(),
                        message: format!("{e:#}"),
                    }));
                }
            }
        });
        Some(id)
    }
}
