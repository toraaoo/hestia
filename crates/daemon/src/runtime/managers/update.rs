use std::sync::Arc;

use engine::Engine;
use proto::download::DownloadProgress;
use proto::update::{UpdateCancelledEvent, UpdateDoneEvent, UpdateErrorEvent, UpdateProgressEvent};

use super::job::{progress_event, settle, Cancellations, Runner, Spec};
use crate::runtime::EventHub;

pub struct UpdateManager {
    runner: Runner<String>,
}

impl UpdateManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>, cancellations: Cancellations) -> Self {
        UpdateManager {
            runner: Runner::new(engine, hub, cancellations),
        }
    }

    /// Start the signed installer download off-thread. Returns the job id.
    pub fn start(&self, id: String) -> String {
        let spec = Spec {
            id,
            prefix: "update",
            key: None,
            progress: progress_event(|id, progress: &DownloadProgress| UpdateProgressEvent {
                id,
                progress: progress.clone(),
            }),
            done: settle(|id, (path, version)| UpdateDoneEvent { id, path, version }),
            cancelled: Some(settle(|id, ()| UpdateCancelledEvent { id })),
            error: settle(|id, e| UpdateErrorEvent {
                id,
                error: crate::runtime::engine_error(e),
            }),
        };

        self.runner
            .start(spec, move |engine, reporter| {
                Box::pin(async move {
                    let report = reporter.checked();
                    engine.download_update(&report).await
                })
            })
            .expect("a keyless job is never refused")
    }
}
