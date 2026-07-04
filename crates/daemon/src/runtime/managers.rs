//! Worker managers that run blocking engine jobs off the request path: an install
//! or download answers immediately while progress and the terminal outcome are
//! published through the event hub.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use engine::{Downloader, Engine};
use ipc::protocol::Event;
use proto::download::{DownloadDoneEvent, DownloadErrorEvent, DownloadProgressEvent, DownloadSpec};
use proto::java::{JavaInstallDoneEvent, JavaInstallErrorEvent};

use super::event_hub::EventHub;

fn topic_event<E: proto::Topic + serde::Serialize>(event: &E) -> Event {
    Event {
        topic: E::TOPIC.to_string(),
        payload: serde_json::to_value(event).unwrap_or_default(),
    }
}

fn generate_id(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    format!("{prefix}-{}-{}", std::process::id(), n)
}

pub struct JavaInstallManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    active: Arc<Mutex<HashSet<i32>>>,
}

impl JavaInstallManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>) -> Self {
        JavaInstallManager {
            engine,
            hub,
            active: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Start an install off-thread, one per release line at a time. Returns the
    /// job id, or `None` if that line is already installing.
    pub fn start(&self, major: i32, id: String, force: bool) -> Option<String> {
        let id = if id.is_empty() {
            generate_id("java-install")
        } else {
            id
        };
        {
            let mut active = self.active.lock().unwrap();
            if !active.insert(major) {
                return None;
            }
        }
        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let active = self.active.clone();
        let job_id = id.clone();

        tokio::spawn(async move {
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
                Ok(outcome) => hub.publish(&topic_event(&JavaInstallDoneEvent {
                    id: job_id.clone(),
                    runtime: outcome.runtime,
                    already_installed: outcome.already_installed,
                })),
                Err(e) => hub.publish(&topic_event(&JavaInstallErrorEvent {
                    id: job_id.clone(),
                    message: e.to_string(),
                })),
            }
            active.lock().unwrap().remove(&major);
        });
        Some(id)
    }
}

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
        if spec.id.is_empty() {
            spec.id = generate_id("download");
        }
        let id = spec.id.clone();
        let job_id = id.clone();
        let engine = self.engine.clone();
        let hub = self.hub.clone();

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
                Ok(()) => hub.publish(&topic_event(&DownloadDoneEvent {
                    id: job_id.clone(),
                    path: spec.destination.clone(),
                })),
                Err(e) => hub.publish(&topic_event(&DownloadErrorEvent {
                    id: job_id.clone(),
                    message: e.to_string(),
                })),
            }
        });
        id
    }
}
