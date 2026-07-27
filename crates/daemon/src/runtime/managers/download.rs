use std::sync::Arc;

use engine::{Downloader, Engine};
use proto::download::{
    DownloadCancelledEvent, DownloadDoneEvent, DownloadErrorEvent, DownloadProgressEvent,
    DownloadSpec,
};

use super::job::{job_id, topic_event, Cancellations};
use crate::runtime::EventHub;

pub struct DownloadManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    cancellations: Cancellations,
}

impl DownloadManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>, cancellations: Cancellations) -> Self {
        DownloadManager {
            engine,
            hub,
            cancellations,
        }
    }

    /// Start a download off-thread. Returns the job id.
    pub fn start(&self, mut spec: DownloadSpec) -> String {
        spec.id = job_id(spec.id, "download");
        let id = spec.id.clone();
        let job_id = id.clone();
        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let cancellations = self.cancellations.clone();
        tracing::info!(job = %id, url = %spec.url, "download started");

        tokio::spawn(async move {
            let (cancel, _registered) = cancellations.register(&job_id);
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress = move |p: &proto::download::DownloadProgress| {
                // Per chunk, so a large download stops promptly; its `.part` is
                // discarded by the failure path rather than promoted.
                cancel.check()?;
                progress_hub.publish(&topic_event(&DownloadProgressEvent {
                    id: progress_id.clone(),
                    progress: p.clone(),
                }));
                Ok(())
            };

            let checksum = spec.checksum.clone();
            let result = Downloader::new(Some(engine.cache()))
                .fetch(
                    &spec.url,
                    &spec.destination,
                    checksum.as_ref(),
                    &on_progress,
                )
                .await;

            match result {
                Ok(()) => {
                    tracing::info!(job = %job_id, path = %spec.destination.display(), "download done");
                    hub.publish(&topic_event(&DownloadDoneEvent {
                        id: job_id.clone(),
                        path: spec.destination.clone(),
                    }));
                }
                Err(e) if engine::is_cancelled(&e) => {
                    tracing::info!(job = %job_id, url = %spec.url, "download cancelled");
                    hub.publish(&topic_event(&DownloadCancelledEvent { id: job_id.clone() }));
                }
                Err(e) => {
                    tracing::error!(job = %job_id, url = %spec.url, error = format!("{e:#}"), "download failed");
                    hub.publish(&topic_event(&DownloadErrorEvent {
                        id: job_id.clone(),
                        error: crate::runtime::engine_error(e),
                    }));
                }
            }
        });
        id
    }
}
