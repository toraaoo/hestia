use std::sync::Arc;

use engine::{Downloader, Engine};
use proto::download::{DownloadDoneEvent, DownloadErrorEvent, DownloadProgressEvent, DownloadSpec};

use super::job::{job_id, topic_event};
use crate::runtime::EventHub;

pub struct DownloadManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
}

impl DownloadManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>) -> Self {
        DownloadManager { engine, hub }
    }

    /// Start a download off-thread. Returns the job id.
    pub fn start(&self, mut spec: DownloadSpec) -> String {
        spec.id = job_id(spec.id, "download");
        let id = spec.id.clone();
        let job_id = id.clone();
        let engine = self.engine.clone();
        let hub = self.hub.clone();
        tracing::info!(job = %id, url = %spec.url, "download started");

        tokio::spawn(async move {
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress = move |p: &proto::download::DownloadProgress| {
                progress_hub.publish(&topic_event(&DownloadProgressEvent {
                    id: progress_id.clone(),
                    progress: p.clone(),
                }));
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
                Err(e) => {
                    tracing::error!(job = %job_id, url = %spec.url, error = format!("{e:#}"), "download failed");
                    hub.publish(&topic_event(&DownloadErrorEvent {
                        id: job_id.clone(),
                        message: e.to_string(),
                    }));
                }
            }
        });
        id
    }
}
