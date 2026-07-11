//! The daemon's long-lived collaborators in one place — the anti-churn seam a
//! new subsystem hangs off, mirroring the engine's aggregate root.

mod event_hub;
mod managers;
mod process;
pub mod router;
mod scheduler;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use engine::{Engine, InstanceRecord, ServerRecord};
use ipc::Peer;
use proto::instance::InstanceInfo;
use proto::server::ServerInfo;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Notify;

pub use event_hub::EventHub;
pub use managers::{
    BackupJob, BackupManager, ContentJob, ContentManager, DownloadManager, InstanceLaunchManager,
    JavaInstallManager, ServerCreateManager, ServerUpdateManager, UpdateManager,
};
pub use process::{ProcessSupervisor, StartError};
pub use router::{Channels, Router, ServiceError};
pub use scheduler::spawn_backup_scheduler;

/// The supervisor id a managed server runs under — deterministic, so every
/// channel can find a server's process without bookkeeping.
pub fn server_process_id(id: &str) -> String {
    format!("server-{id}")
}

pub fn instance_process_id(id: &str) -> String {
    format!("instance-{id}")
}

pub fn server_info(
    record: ServerRecord,
    process: Option<proto::process::ProcessInfo>,
) -> ServerInfo {
    ServerInfo {
        id: record.id,
        name: record.name,
        flavor: record.profile.flavor,
        game_version: record.profile.game_version,
        loader_version: record.profile.loader_version,
        java_major: record.profile.java_major,
        created_unix: record.created_unix,
        ready: record.ready,
        game_port: record.game_port,
        console: record.rcon.is_some(),
        process,
    }
}

pub fn instance_info(
    record: InstanceRecord,
    process: Option<proto::process::ProcessInfo>,
) -> InstanceInfo {
    InstanceInfo {
        id: record.id,
        name: record.name,
        flavor: record.profile.flavor,
        game_version: record.profile.game_version,
        loader_version: record.profile.loader_version,
        java_major: record.profile.java_major,
        created_unix: record.created_unix,
        process,
    }
}

pub struct Runtime {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    java_installs: JavaInstallManager,
    downloads: DownloadManager,
    server_creates: ServerCreateManager,
    server_updates: ServerUpdateManager,
    instance_launches: InstanceLaunchManager,
    backups: BackupManager,
    content_jobs: ContentManager,
    updates: UpdateManager,
    processes: Arc<ProcessSupervisor>,
    log_path: PathBuf,
    started: Instant,
    stop: Notify,
    stop_processes: AtomicBool,
}

impl Runtime {
    pub fn new(log_path: PathBuf, override_home: Option<&std::path::Path>) -> Self {
        let engine = Arc::new(Engine::new(override_home));
        let hub = Arc::new(EventHub::default());
        let java_installs = JavaInstallManager::new(engine.clone(), hub.clone());
        let downloads = DownloadManager::new(engine.clone(), hub.clone());
        let processes = Arc::new(ProcessSupervisor::new(
            hub.clone(),
            engine.data_home().join("processes"),
        ));
        let server_creates = ServerCreateManager::new(engine.clone(), hub.clone());
        let server_updates = ServerUpdateManager::new(engine.clone(), hub.clone());
        let instance_launches =
            InstanceLaunchManager::new(engine.clone(), hub.clone(), processes.clone());
        let backups = BackupManager::new(engine.clone(), hub.clone());
        let content_jobs = ContentManager::new(engine.clone(), hub.clone());
        let updates = UpdateManager::new(engine.clone(), hub.clone());
        Runtime {
            engine,
            hub,
            java_installs,
            downloads,
            server_creates,
            server_updates,
            instance_launches,
            backups,
            content_jobs,
            updates,
            processes,
            log_path,
            started: Instant::now(),
            stop: Notify::new(),
            stop_processes: AtomicBool::new(false),
        }
    }

    /// A server's record merged with its live process state (when started).
    pub fn server_view(&self, record: ServerRecord) -> ServerInfo {
        let process = self.processes.status(&server_process_id(&record.id));
        server_info(record, process)
    }

    pub fn instance_view(&self, record: InstanceRecord) -> InstanceInfo {
        let process = self.processes.status(&instance_process_id(&record.id));
        instance_info(record, process)
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

    pub fn server_creates(&self) -> &ServerCreateManager {
        &self.server_creates
    }

    pub fn server_updates(&self) -> &ServerUpdateManager {
        &self.server_updates
    }

    pub fn instance_launches(&self) -> &InstanceLaunchManager {
        &self.instance_launches
    }

    pub fn backups(&self) -> &BackupManager {
        &self.backups
    }

    pub fn content_jobs(&self) -> &ContentManager {
        &self.content_jobs
    }

    pub fn updates(&self) -> &UpdateManager {
        &self.updates
    }

    pub fn processes(&self) -> &ProcessSupervisor {
        &self.processes
    }

    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }

    pub fn uptime_seconds(&self) -> i64 {
        self.started.elapsed().as_secs() as i64
    }

    /// Ask the serve loop to shut down (the `daemon.stop` handler calls this).
    pub fn request_stop(&self, stop_processes: bool) {
        self.stop_processes.store(stop_processes, Ordering::SeqCst);
        self.stop.notify_waiters();
    }

    /// Resolves when a stop has been requested.
    pub async fn stopped(&self) {
        self.stop.notified().await;
    }

    /// An OS-signal shutdown never stops workloads; only an explicit
    /// `daemon.stop` with `stop_processes` does.
    pub async fn shutdown_workloads(&self) {
        if self.stop_processes.load(Ordering::SeqCst) {
            self.processes.stop_all_and_wait().await;
        }
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
