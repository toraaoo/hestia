use std::path::PathBuf;
use std::sync::Arc;

use engine::{Downloader, Engine};
use proto::download::{
    DownloadCancelledEvent, DownloadDoneEvent, DownloadErrorEvent, DownloadProgress,
    DownloadProgressEvent, DownloadSpec,
};

use super::job::{progress_event, settle, Cancellations, Runner, Spec};
use crate::runtime::EventHub;

pub struct DownloadManager {
    runner: Runner<String>,
}

impl DownloadManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>, cancellations: Cancellations) -> Self {
        DownloadManager {
            runner: Runner::new(engine, hub, cancellations),
        }
    }

    /// Start a download off-thread. Returns the job id.
    pub fn start(&self, spec: DownloadSpec) -> String {
        let job = Spec {
            id: spec.id.clone(),
            prefix: "download",
            key: None,
            progress: progress_event(|id, progress: &DownloadProgress| DownloadProgressEvent {
                id,
                progress: progress.clone(),
            }),
            done: settle(|id, path: PathBuf| DownloadDoneEvent { id, path }),
            cancelled: Some(settle(|id, ()| DownloadCancelledEvent { id })),
            error: settle(|id, e| DownloadErrorEvent {
                id,
                error: crate::runtime::engine_error(e),
            }),
        };

        self.runner
            .start(job, move |engine, reporter| {
                Box::pin(async move {
                    // Per chunk, so a large download stops promptly; its `.part`
                    // is discarded by the failure path rather than promoted.
                    let report = reporter.checked();
                    Downloader::new(Some(engine.cache()))
                        .fetch(
                            &spec.url,
                            &spec.destination,
                            spec.checksum.as_ref(),
                            &report,
                        )
                        .await
                        .map(|()| spec.destination.clone())
                })
            })
            .expect("a keyless job is never refused")
    }
}
