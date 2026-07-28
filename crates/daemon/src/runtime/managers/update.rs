use std::sync::Arc;

use engine::Engine;
use proto::update::{UpdateDoneEvent, UpdateErrorEvent, UpdateProgressEvent};

use super::job::{job_id, topic_event};
use super::Cancellations;
use crate::runtime::EventHub;

pub struct UpdateManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    cancellations: Cancellations,
}

impl UpdateManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>, cancellations: Cancellations) -> Self {
        UpdateManager {
            engine,
            hub,
            cancellations,
        }
    }

    /// Start the signed installer download off-thread. Returns the job id.
    pub fn start(&self, id: String) -> String {
        let id = job_id(id, "update");
        let job_id = id.clone();
        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let cancellations = self.cancellations.clone();
        tracing::info!(job = %id, "update download started");

        tokio::spawn(async move {
            let (cancel, _registered) = cancellations.register(&job_id);
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress = move |p: &proto::download::DownloadProgress| {
                cancel.check()?;
                progress_hub.publish(&topic_event(&UpdateProgressEvent {
                    id: progress_id.clone(),
                    progress: p.clone(),
                }));
                Ok(())
            };

            match engine.update().download(&on_progress).await {
                Ok((path, version)) => {
                    tracing::info!(job = %job_id, path = %path.display(), version, "update download done");
                    hub.publish(&topic_event(&UpdateDoneEvent {
                        id: job_id.clone(),
                        path,
                        version,
                    }));
                }
                Err(e) => {
                    tracing::error!(job = %job_id, error = format!("{e:#}"), "update download failed");
                    hub.publish(&topic_event(&UpdateErrorEvent {
                        id: job_id.clone(),
                        message: format!("{e:#}"),
                    }));
                }
            }
        });
        id
    }
}
