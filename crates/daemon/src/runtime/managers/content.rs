use std::sync::Arc;

use engine::Engine;
use proto::content::{
    ContentAddSpec, ContentCancelledEvent, ContentDoneEvent, ContentErrorEvent, ContentFailure,
    ContentKind, ContentProgressEvent, InstalledContent,
};
use proto::error::EntryKind;
use proto::minecraft::ProvisionProgress;

use super::job::{progress_event, settle, Cancellations, Runner, Spec};
use crate::runtime::{instance_process_id, server_process_id, EventHub};

/// One content install or update for one entry — what `ContentManager::start`
/// runs off-thread. The side is a field, not a second copy of every verb.
pub enum ContentJob {
    Add {
        entry: Entry,
        spec: ContentAddSpec,
    },
    Update {
        entry: Entry,
        kind: ContentKind,
        item: String,
    },
    /// Re-pin one item to a specific published version.
    SetVersion {
        entry: Entry,
        kind: ContentKind,
        item: String,
        version: String,
    },
    /// Apply a global profile's references into an instance's pool.
    ProfileApply {
        instance_id: String,
        profile: String,
    },
}

/// Which entry a job is for, owned — the job outlives the request that asked
/// for it, so it cannot borrow the reference.
pub struct Entry {
    pub kind: EntryKind,
    pub id: String,
}

impl Entry {
    pub fn server(id: String) -> Entry {
        Entry {
            kind: EntryKind::Server,
            id,
        }
    }

    pub fn instance(id: String) -> Entry {
        Entry {
            kind: EntryKind::Instance,
            id,
        }
    }

    fn as_ref(&self) -> engine::EntryRef<'_> {
        match self.kind {
            EntryKind::Server => engine::EntryRef::Server(&self.id),
            EntryKind::Instance => engine::EntryRef::Instance(&self.id),
        }
    }

    fn key(&self) -> String {
        match self.kind {
            EntryKind::Server => server_process_id(&self.id),
            EntryKind::Instance => instance_process_id(&self.id),
        }
    }
}

type Installed = (Vec<InstalledContent>, Vec<ContentFailure>);

impl ContentJob {
    /// The in-flight key: one content change per entry at a time, keyed by the
    /// entry's process id like the backup jobs.
    fn key(&self) -> String {
        match self {
            ContentJob::Add { entry, .. }
            | ContentJob::Update { entry, .. }
            | ContentJob::SetVersion { entry, .. } => entry.key(),
            ContentJob::ProfileApply { instance_id, .. } => instance_process_id(instance_id),
        }
    }

    fn id_prefix(&self) -> &'static str {
        match self {
            ContentJob::Add { entry, .. } => match entry.kind {
                EntryKind::Server => "server-content-add",
                EntryKind::Instance => "instance-content-add",
            },
            ContentJob::Update { entry, .. } => match entry.kind {
                EntryKind::Server => "server-content-update",
                EntryKind::Instance => "instance-content-update",
            },
            ContentJob::SetVersion { entry, .. } => match entry.kind {
                EntryKind::Server => "server-content-set-version",
                EntryKind::Instance => "instance-content-set-version",
            },
            ContentJob::ProfileApply { .. } => "profile-apply",
        }
    }

    async fn run(
        self,
        engine: &Engine,
        on_progress: &engine::Job<'_>,
    ) -> anyhow::Result<Installed> {
        match self {
            ContentJob::Add { entry, spec } => {
                engine
                    .add_entry_content(entry.as_ref(), &spec, on_progress)
                    .await
            }
            ContentJob::Update { entry, kind, item } => engine
                .update_entry_content(entry.as_ref(), kind, &item, on_progress)
                .await
                .map(|items| (items, Vec::new())),
            ContentJob::SetVersion {
                entry,
                kind,
                item,
                version,
            } => engine
                .set_entry_content_version(entry.as_ref(), kind, &item, &version, on_progress)
                .await
                .map(|items| (items, Vec::new())),
            ContentJob::ProfileApply {
                instance_id,
                profile,
            } => {
                engine
                    .apply_global_profile(&instance_id, &profile, on_progress)
                    .await
            }
        }
    }
}

pub struct ContentManager {
    runner: Runner<String>,
}

impl ContentManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>, cancellations: Cancellations) -> Self {
        ContentManager {
            runner: Runner::new(engine, hub, cancellations),
        }
    }

    /// Whether a content change is still running for this entry key
    /// (`server-<id>` / `instance-<id>`).
    pub fn in_flight(&self, key: &str) -> bool {
        self.runner.in_flight(key)
    }

    /// Start an install/update job off-thread, one per entry at a time.
    /// Returns the job id, or `None` if that entry is already busy.
    pub fn start(&self, job: ContentJob, id: String) -> Option<String> {
        let spec = Spec {
            id,
            prefix: job.id_prefix(),
            key: Some(job.key()),
            progress: progress_event(|id, progress: &ProvisionProgress| ContentProgressEvent {
                id,
                progress: progress.clone(),
            }),
            done: settle(|id, (items, failures): Installed| ContentDoneEvent {
                id,
                items,
                failures,
            }),
            cancelled: Some(settle(|id, ()| ContentCancelledEvent { id })),
            error: settle(|id, e| ContentErrorEvent {
                id,
                error: crate::runtime::engine_error(e),
            }),
        };

        self.runner.start(spec, move |engine, reporter| {
            Box::pin(async move { job.run(engine, &reporter.job()).await })
        })
    }
}
