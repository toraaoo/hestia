//! Worker managers that run blocking engine jobs off the request path: an install
//! or download answers immediately while progress and the terminal outcome are
//! published through the event hub.

use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use engine::{Downloader, Engine, ServerCreateSpec, ServerUpdateSpec};
use ipc::protocol::Event;
use proto::backup::{
    BackupDoneEvent, BackupErrorEvent, BackupInfo, BackupKind, BackupProgressEvent,
};
use proto::content::{
    ContentAddSpec, ContentDoneEvent, ContentErrorEvent, ContentKind, ContentProgressEvent,
    InstalledContent,
};
use proto::download::{DownloadDoneEvent, DownloadErrorEvent, DownloadProgressEvent, DownloadSpec};
use proto::instance::{
    InstanceLaunchDoneEvent, InstanceLaunchErrorEvent, InstanceLaunchProgressEvent,
};
use proto::java::{JavaInstallDoneEvent, JavaInstallErrorEvent};
use proto::minecraft::ProvisionProgress;
use proto::process::{LogSource, ProcessSpec, RestartPolicy};
use proto::server::{
    ServerCreateDoneEvent, ServerCreateErrorEvent, ServerCreateParams, ServerCreateProgressEvent,
    ServerUpdateDoneEvent, ServerUpdateErrorEvent, ServerUpdateParams, ServerUpdateProgressEvent,
};

use super::event_hub::EventHub;
use super::process::{ProcessSupervisor, StartError};
use super::{instance_process_id, server_info, server_process_id};

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
                tracing::debug!(major, "java install already in flight");
                return None;
            }
        }
        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let active = self.active.clone();
        let job_id = id.clone();
        tracing::info!(job = %id, major, force, "java install started");

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
            active.lock().unwrap().remove(&major);
        });
        Some(id)
    }
}

pub struct ServerCreateManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    active: Arc<Mutex<HashSet<String>>>,
}

impl ServerCreateManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>) -> Self {
        ServerCreateManager {
            engine,
            hub,
            active: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Whether a create for this server name is still provisioning.
    pub fn in_flight(&self, name: &str) -> bool {
        self.active.lock().unwrap().contains(name)
    }

    /// Start a provisioning job off-thread, one per server name at a time.
    /// Returns the job id, or `None` if that name is already being created.
    pub fn start(&self, params: ServerCreateParams) -> Option<String> {
        let id = if params.id.is_empty() {
            generate_id("server-create")
        } else {
            params.id.clone()
        };
        let key = if params.name.trim().is_empty() {
            format!("{}-{}", params.flavor, params.version)
        } else {
            params.name.trim().to_string()
        };
        {
            let mut active = self.active.lock().unwrap();
            if !active.insert(key.clone()) {
                tracing::debug!(server = %key, "server create already in flight");
                return None;
            }
        }
        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let active = self.active.clone();
        let job_id = id.clone();
        tracing::info!(
            job = %id,
            name = %params.name,
            flavor = %params.flavor,
            version = %params.version,
            "server create started"
        );

        tokio::spawn(async move {
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress: Box<dyn Fn(&ProvisionProgress) + Send + Sync> = Box::new(move |p| {
                progress_hub.publish(&topic_event(&ServerCreateProgressEvent {
                    id: progress_id.clone(),
                    progress: p.clone(),
                }));
            });

            let spec = ServerCreateSpec {
                name: params.name,
                flavor: params.flavor,
                version: params.version,
                loader_version: params.loader_version,
                port: params.port,
                config: params.config,
            };
            let result = engine.provision_server(spec, on_progress.as_ref()).await;

            match result {
                Ok(record) => {
                    tracing::info!(
                        job = %job_id,
                        server = %record.id,
                        name = %record.name,
                        "server create done"
                    );
                    hub.publish(&topic_event(&ServerCreateDoneEvent {
                        id: job_id.clone(),
                        server: server_info(record, None),
                    }));
                }
                Err(e) => {
                    tracing::error!(job = %job_id, error = format!("{e:#}"), "server create failed");
                    hub.publish(&topic_event(&ServerCreateErrorEvent {
                        id: job_id.clone(),
                        message: format!("{e:#}"),
                    }));
                }
            }
            active.lock().unwrap().remove(&key);
        });
        Some(id)
    }
}

pub struct ServerUpdateManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    active: Arc<Mutex<HashSet<String>>>,
}

impl ServerUpdateManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>) -> Self {
        ServerUpdateManager {
            engine,
            hub,
            active: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Whether an update for this server id is still running.
    pub fn in_flight(&self, server_id: &str) -> bool {
        self.active.lock().unwrap().contains(server_id)
    }

    /// Start an update job off-thread, one per server at a time. Returns the
    /// job id, or `None` if that server is already being updated.
    pub fn start(&self, server_id: String, params: ServerUpdateParams) -> Option<String> {
        let id = if params.id.is_empty() {
            generate_id("server-update")
        } else {
            params.id.clone()
        };
        {
            let mut active = self.active.lock().unwrap();
            if !active.insert(server_id.clone()) {
                tracing::debug!(server = %server_id, "server update already in flight");
                return None;
            }
        }
        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let active = self.active.clone();
        let job_id = id.clone();
        tracing::info!(
            job = %id,
            server = %server_id,
            version = %params.version,
            allow_downgrade = params.allow_downgrade,
            "server update started"
        );

        tokio::spawn(async move {
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress: Box<dyn Fn(&ProvisionProgress) + Send + Sync> = Box::new(move |p| {
                progress_hub.publish(&topic_event(&ServerUpdateProgressEvent {
                    id: progress_id.clone(),
                    progress: p.clone(),
                }));
            });

            let spec = ServerUpdateSpec {
                server: server_id.clone(),
                version: params.version,
                loader_version: params.loader_version,
                allow_downgrade: params.allow_downgrade,
            };
            let result = engine.update_server(spec, on_progress.as_ref()).await;

            match result {
                Ok(record) => {
                    tracing::info!(
                        job = %job_id,
                        server = %record.id,
                        version = %record.profile.game_version,
                        "server update done"
                    );
                    hub.publish(&topic_event(&ServerUpdateDoneEvent {
                        id: job_id.clone(),
                        server: server_info(record, None),
                    }));
                }
                Err(e) => {
                    tracing::error!(job = %job_id, server = %server_id, error = format!("{e:#}"), "server update failed");
                    hub.publish(&topic_event(&ServerUpdateErrorEvent {
                        id: job_id.clone(),
                        message: format!("{e:#}"),
                    }));
                }
            }
            active.lock().unwrap().remove(&server_id);
        });
        Some(id)
    }
}

pub struct InstanceLaunchManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    processes: Arc<ProcessSupervisor>,
    active: Arc<Mutex<HashSet<String>>>,
}

impl InstanceLaunchManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>, processes: Arc<ProcessSupervisor>) -> Self {
        InstanceLaunchManager {
            engine,
            hub,
            processes,
            active: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Prepare and spawn an instance off-thread, one launch per instance at a
    /// time. Returns the job id, or `None` if that instance is already
    /// launching.
    pub fn start(&self, instance_id: String, account: String, id: String) -> Option<String> {
        let id = if id.is_empty() {
            generate_id("instance-launch")
        } else {
            id
        };
        {
            let mut active = self.active.lock().unwrap();
            if !active.insert(instance_id.clone()) {
                tracing::debug!(instance = %instance_id, "instance launch already in flight");
                return None;
            }
        }
        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let processes = self.processes.clone();
        let active = self.active.clone();
        let job_id = id.clone();
        tracing::info!(job = %id, instance = %instance_id, account = %account, "instance launch started");

        tokio::spawn(async move {
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress: Box<dyn Fn(&ProvisionProgress) + Send + Sync> = Box::new(move |p| {
                progress_hub.publish(&topic_event(&InstanceLaunchProgressEvent {
                    id: progress_id.clone(),
                    progress: p.clone(),
                }));
            });

            let outcome = launch(
                &engine,
                &processes,
                &instance_id,
                &account,
                on_progress.as_ref(),
            )
            .await;
            match outcome {
                Ok((process_id, pid)) => {
                    tracing::info!(job = %job_id, process = %process_id, pid, "instance launch done");
                    hub.publish(&topic_event(&InstanceLaunchDoneEvent {
                        id: job_id.clone(),
                        process_id,
                        pid,
                    }));
                }
                Err(message) => {
                    tracing::error!(job = %job_id, instance = %instance_id, error = %message, "instance launch failed");
                    hub.publish(&topic_event(&InstanceLaunchErrorEvent {
                        id: job_id.clone(),
                        message,
                    }));
                }
            }
            active.lock().unwrap().remove(&instance_id);
        });
        Some(id)
    }
}

/// Materialise the instance, then hand the plan to the supervisor.
async fn launch(
    engine: &Engine,
    processes: &ProcessSupervisor,
    instance_id: &str,
    account: &str,
    on_progress: &(dyn Fn(&ProvisionProgress) + Send + Sync),
) -> Result<(String, u32), String> {
    let (record, plan) = engine
        .prepare_instance(instance_id, account, on_progress)
        .await
        .map_err(|e| format!("{e:#}"))?;

    let spec = ProcessSpec {
        id: instance_process_id(&record.id),
        program: plan.program.to_string_lossy().into_owned(),
        args: plan.args,
        log: LogSource::File(plan.cwd.join("logs").join("latest.log")),
        cwd: Some(plan.cwd),
        env: BTreeMap::new(),
        restart: RestartPolicy::Never,
    };
    match processes.start(spec).await {
        Ok(info) => Ok((info.id, info.pid)),
        Err(StartError::EmptyProgram | StartError::InvalidId) => {
            Err("invalid launch plan".to_string())
        }
        Err(StartError::Spawn(e)) => Err(format!("cannot spawn the game: {e}")),
    }
}

/// One backup or restore job for one entry — what `BackupManager::start`
/// runs off-thread.
pub enum BackupJob {
    ServerBackup { server_id: String, live: bool },
    ServerRestore { server_id: String, backup: String },
    InstanceBackup { instance_id: String },
    InstanceRestore { instance_id: String, backup: String },
}

impl BackupJob {
    /// The in-flight key: one backup *or* restore per entry at a time. The
    /// entry's process id is the key, so handlers can check it without
    /// re-deriving a format.
    fn key(&self) -> String {
        match self {
            BackupJob::ServerBackup { server_id, .. }
            | BackupJob::ServerRestore { server_id, .. } => server_process_id(server_id),
            BackupJob::InstanceBackup { instance_id }
            | BackupJob::InstanceRestore { instance_id, .. } => instance_process_id(instance_id),
        }
    }

    fn id_prefix(&self) -> &'static str {
        match self {
            BackupJob::ServerBackup { .. } => "server-backup",
            BackupJob::ServerRestore { .. } => "server-restore",
            BackupJob::InstanceBackup { .. } => "instance-backup",
            BackupJob::InstanceRestore { .. } => "instance-restore",
        }
    }

    async fn run(
        self,
        engine: &Engine,
        on_progress: &(dyn Fn(&ProvisionProgress) + Send + Sync),
    ) -> anyhow::Result<BackupInfo> {
        match self {
            BackupJob::ServerBackup { server_id, live } => {
                engine
                    .backup_server(&server_id, BackupKind::Manual, live, on_progress)
                    .await
            }
            BackupJob::ServerRestore { server_id, backup } => {
                engine
                    .restore_server_backup(&server_id, &backup, on_progress)
                    .await
            }
            BackupJob::InstanceBackup { instance_id } => {
                engine
                    .backup_instance(&instance_id, BackupKind::Manual, on_progress)
                    .await
            }
            BackupJob::InstanceRestore {
                instance_id,
                backup,
            } => {
                engine
                    .restore_instance_backup(&instance_id, &backup, on_progress)
                    .await
            }
        }
    }
}

pub struct BackupManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    active: Arc<Mutex<HashSet<String>>>,
}

impl BackupManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>) -> Self {
        BackupManager {
            engine,
            hub,
            active: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Whether a backup or restore is still running for this entry key
    /// (`server-<id>` / `instance-<id>`).
    pub fn in_flight(&self, key: &str) -> bool {
        self.active.lock().unwrap().contains(key)
    }

    /// Start a backup/restore job off-thread, one per entry at a time.
    /// Returns the job id, or `None` if that entry is already busy.
    pub fn start(&self, job: BackupJob, id: String) -> Option<String> {
        let id = if id.is_empty() {
            generate_id(job.id_prefix())
        } else {
            id
        };
        let key = job.key();
        {
            let mut active = self.active.lock().unwrap();
            if !active.insert(key.clone()) {
                tracing::debug!(entry = %key, "backup job already in flight");
                return None;
            }
        }
        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let active = self.active.clone();
        let job_id = id.clone();
        tracing::info!(job = %id, entry = %key, kind = job.id_prefix(), "backup job started");

        tokio::spawn(async move {
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress: Box<dyn Fn(&ProvisionProgress) + Send + Sync> = Box::new(move |p| {
                progress_hub.publish(&topic_event(&BackupProgressEvent {
                    id: progress_id.clone(),
                    progress: p.clone(),
                }));
            });

            match job.run(&engine, on_progress.as_ref()).await {
                Ok(backup) => {
                    tracing::info!(job = %job_id, backup = %backup.id, size = backup.size, "backup job done");
                    hub.publish(&topic_event(&BackupDoneEvent {
                        id: job_id.clone(),
                        backup,
                    }));
                }
                Err(e) => {
                    tracing::error!(job = %job_id, error = format!("{e:#}"), "backup job failed");
                    hub.publish(&topic_event(&BackupErrorEvent {
                        id: job_id.clone(),
                        message: format!("{e:#}"),
                    }));
                }
            }
            active.lock().unwrap().remove(&key);
        });
        Some(id)
    }
}

/// One content install or update for one entry — what `ContentManager::start`
/// runs off-thread.
pub enum ContentJob {
    ServerAdd {
        server_id: String,
        spec: ContentAddSpec,
    },
    InstanceAdd {
        instance_id: String,
        spec: ContentAddSpec,
    },
    ServerUpdate {
        server_id: String,
        kind: ContentKind,
        item: String,
    },
    InstanceUpdate {
        instance_id: String,
        kind: ContentKind,
        item: String,
    },
}

impl ContentJob {
    /// The in-flight key: one content change per entry at a time, keyed by the
    /// entry's process id like the backup jobs.
    fn key(&self) -> String {
        match self {
            ContentJob::ServerAdd { server_id, .. }
            | ContentJob::ServerUpdate { server_id, .. } => server_process_id(server_id),
            ContentJob::InstanceAdd { instance_id, .. }
            | ContentJob::InstanceUpdate { instance_id, .. } => instance_process_id(instance_id),
        }
    }

    fn id_prefix(&self) -> &'static str {
        match self {
            ContentJob::ServerAdd { .. } => "server-content-add",
            ContentJob::InstanceAdd { .. } => "instance-content-add",
            ContentJob::ServerUpdate { .. } => "server-content-update",
            ContentJob::InstanceUpdate { .. } => "instance-content-update",
        }
    }

    async fn run(
        self,
        engine: &Engine,
        on_progress: &(dyn Fn(&ProvisionProgress) + Send + Sync),
    ) -> anyhow::Result<Vec<InstalledContent>> {
        match self {
            ContentJob::ServerAdd { server_id, spec } => {
                engine
                    .add_server_content(&server_id, &spec, on_progress)
                    .await
            }
            ContentJob::InstanceAdd { instance_id, spec } => {
                engine
                    .add_instance_content(&instance_id, &spec, on_progress)
                    .await
            }
            ContentJob::ServerUpdate {
                server_id,
                kind,
                item,
            } => {
                engine
                    .update_server_content(&server_id, kind, &item, on_progress)
                    .await
            }
            ContentJob::InstanceUpdate {
                instance_id,
                kind,
                item,
            } => {
                engine
                    .update_instance_content(&instance_id, kind, &item, on_progress)
                    .await
            }
        }
    }
}

pub struct ContentManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    active: Arc<Mutex<HashSet<String>>>,
}

impl ContentManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>) -> Self {
        ContentManager {
            engine,
            hub,
            active: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Whether a content change is still running for this entry key
    /// (`server-<id>` / `instance-<id>`).
    pub fn in_flight(&self, key: &str) -> bool {
        self.active.lock().unwrap().contains(key)
    }

    /// Start an install/update job off-thread, one per entry at a time.
    /// Returns the job id, or `None` if that entry is already busy.
    pub fn start(&self, job: ContentJob, id: String) -> Option<String> {
        let id = if id.is_empty() {
            generate_id(job.id_prefix())
        } else {
            id
        };
        let key = job.key();
        {
            let mut active = self.active.lock().unwrap();
            if !active.insert(key.clone()) {
                tracing::debug!(entry = %key, "content job already in flight");
                return None;
            }
        }
        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let active = self.active.clone();
        let job_id = id.clone();
        tracing::info!(job = %id, entry = %key, kind = job.id_prefix(), "content job started");

        tokio::spawn(async move {
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress: Box<dyn Fn(&ProvisionProgress) + Send + Sync> = Box::new(move |p| {
                progress_hub.publish(&topic_event(&ContentProgressEvent {
                    id: progress_id.clone(),
                    progress: p.clone(),
                }));
            });

            match job.run(&engine, on_progress.as_ref()).await {
                Ok(items) => {
                    tracing::info!(job = %job_id, items = items.len(), "content job done");
                    hub.publish(&topic_event(&ContentDoneEvent {
                        id: job_id.clone(),
                        items,
                    }));
                }
                Err(e) => {
                    tracing::error!(job = %job_id, error = format!("{e:#}"), "content job failed");
                    hub.publish(&topic_event(&ContentErrorEvent {
                        id: job_id.clone(),
                        message: format!("{e:#}"),
                    }));
                }
            }
            active.lock().unwrap().remove(&key);
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
