//! The daemon's long-lived collaborators in one place — the anti-churn seam a
//! new subsystem hangs off, mirroring the engine's aggregate root.

mod event_hub;
mod managers;
pub mod router;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use engine::Engine;
use ipc::Peer;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Notify;

pub use event_hub::EventHub;
pub use managers::{DownloadManager, JavaInstallManager};
pub use router::{Channels, Router, ServiceError};

pub struct Runtime {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    java_installs: JavaInstallManager,
    downloads: DownloadManager,
    log_path: PathBuf,
    started: Instant,
    stop: Notify,
}

impl Runtime {
    pub fn new(log_path: PathBuf, override_home: Option<&std::path::Path>) -> Self {
        let engine = Arc::new(Engine::new(override_home));
        let hub = Arc::new(EventHub::default());
        let java_installs = JavaInstallManager::new(engine.clone(), hub.clone());
        let downloads = DownloadManager::new(engine.clone(), hub.clone());
        Runtime {
            engine,
            hub,
            java_installs,
            downloads,
            log_path,
            started: Instant::now(),
            stop: Notify::new(),
        }
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn hub(&self) -> &EventHub {
        &self.hub
    }

    pub fn java_installs(&self) -> &JavaInstallManager {
        &self.java_installs
    }

    pub fn downloads(&self) -> &DownloadManager {
        &self.downloads
    }

    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }

    pub fn uptime_seconds(&self) -> i64 {
        self.started.elapsed().as_secs() as i64
    }

    /// Ask the serve loop to shut down (the `daemon.stop` handler calls this).
    pub fn request_stop(&self) {
        self.stop.notify_waiters();
    }

    /// Resolves when a stop has been requested.
    pub async fn stopped(&self) {
        self.stop.notified().await;
    }
}

/// What every handler receives: the shared runtime, the calling connection's
/// outbound channel (so streaming handlers like `events.subscribe` can push to
/// it), and the verified peer.
#[derive(Clone)]
pub struct HandlerContext {
    pub runtime: Arc<Runtime>,
    pub conn_id: u64,
    pub out: UnboundedSender<String>,
    // The verified peer identity: the seam a future token/cert auth check reads.
    // Carried on every request even though no handler consumes it yet.
    #[allow(dead_code)]
    pub peer: Peer,
}
