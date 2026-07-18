//! The daemon's long-lived collaborators in one place — the anti-churn seam a
//! new subsystem hangs off, mirroring the engine's aggregate root.

mod event_hub;
mod managers;
mod metrics;
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
    JavaInstallManager, ServerCreateManager, ServerUpdateManager,
};
pub use metrics::spawn_metrics_sampler;
pub use process::{ProcessSupervisor, StartError};
pub use router::{Channels, Router, ServiceError};
pub use scheduler::spawn_backup_scheduler;

/// The supervisor id a managed server runs under — deterministic, so every
/// channel can find a server's process without bookkeeping.
pub fn server_process_id(id: &str) -> String {
    format!("server-{id}")
}

/// The instance *entry* key — the unit for the backup/content/update in-flight
/// sets and the lifecycle guards. Not a supervisor process key: an instance can
/// have many concurrent sessions, each keyed by `instance_session_id`.
pub fn instance_process_id(id: &str) -> String {
    format!("instance-{id}")
}

/// The supervisor process key for one launch (session) of an instance. An id
/// never contains `_` (it is a slug plus a hex tag, all `[a-z0-9-]`), so the
/// `_` separator keeps the prefix `instance-<id>_` unambiguous across instances.
pub fn instance_session_id(id: &str, seq: u32) -> String {
    format!("instance-{id}_{seq}")
}

/// The prefix every session key of one instance shares.
pub fn instance_session_prefix(id: &str) -> String {
    format!("instance-{id}_")
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
    sessions: Vec<proto::process::ProcessInfo>,
) -> InstanceInfo {
    InstanceInfo {
        id: record.id,
        name: record.name,
        flavor: record.profile.flavor,
        game_version: record.profile.game_version,
        loader_version: record.profile.loader_version,
        java_major: record.profile.java_major,
        created_unix: record.created_unix,
        sessions,
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
        let sessions = self.instance_sessions(&record.id);
        instance_info(record, sessions)
    }

    /// Every live session of an instance, newest first.
    pub fn instance_sessions(&self, id: &str) -> Vec<proto::process::ProcessInfo> {
        let prefix = instance_session_prefix(id);
        let mut sessions: Vec<_> = self
            .processes
            .list()
            .into_iter()
            .filter(|p| p.id.starts_with(&prefix))
            .collect();
        sessions.sort_by_key(|s| std::cmp::Reverse(s.started_unix));
        sessions
    }

    /// True while any session of the instance is still running.
    pub fn instance_running(&self, id: &str) -> bool {
        self.instance_sessions(id)
            .iter()
            .any(|p| p.state == proto::process::ProcessState::Running)
    }

    /// Stop every session of an instance; returns how many were signalled.
    pub fn stop_instance_sessions(&self, id: &str) -> usize {
        self.instance_sessions(id)
            .into_iter()
            .filter(|p| self.processes.stop(&p.id))
            .count()
    }

    /// Discard the supervisor state of every session of an instance.
    pub fn discard_instance_sessions(&self, id: &str) {
        for session in self.instance_sessions(id) {
            self.processes.discard(&session.id);
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

#[cfg(test)]
mod tests {
    use super::{instance_session_id, instance_session_prefix};

    #[test]
    fn a_sessions_prefix_never_matches_a_similarly_named_instance() {
        // Ids are slugs ([a-z0-9-]); using `_` as the session separator keeps
        // one instance's session prefix from matching another's sessions.
        let foo = instance_session_id("foo", 3);
        let foo_two = instance_session_id("foo-2", 1);
        assert!(foo.starts_with(&instance_session_prefix("foo")));
        assert!(!foo_two.starts_with(&instance_session_prefix("foo")));
        assert!(foo_two.starts_with(&instance_session_prefix("foo-2")));
    }

    #[test]
    fn session_seq_parses_back_off_the_prefix() {
        let id = instance_session_id("cozy", 7);
        let seq: u32 = id
            .strip_prefix(&instance_session_prefix("cozy"))
            .and_then(|s| s.parse().ok())
            .unwrap();
        assert_eq!(seq, 7);
    }
}
