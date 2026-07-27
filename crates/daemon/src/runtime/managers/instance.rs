use std::collections::BTreeMap;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use engine::Engine;
use proto::error::ErrorInfo;
use proto::instance::{
    InstanceLaunchCancelledEvent, InstanceLaunchDoneEvent, InstanceLaunchErrorEvent,
    InstanceLaunchProgressEvent,
};
use proto::minecraft::ProvisionProgress;
use proto::process::{LogSource, ProcessSpec, RestartPolicy};
use proto::warning::WarningInfo;

use super::job::{coalesce_progress, job_id, topic_event, Cancellations};
use crate::runtime::{
    instance_session_id, instance_session_prefix, EventHub, ProcessSupervisor, StartError,
};

pub struct InstanceLaunchManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    processes: Arc<ProcessSupervisor>,
    cancellations: Cancellations,
    /// Session ids reserved between seq allocation and the supervisor accepting
    /// them, so two concurrent launches of one instance can't collide on a seq.
    reserved: Arc<Mutex<HashSet<String>>>,
}

impl InstanceLaunchManager {
    pub fn new(
        engine: Arc<Engine>,
        hub: Arc<EventHub>,
        processes: Arc<ProcessSupervisor>,
        cancellations: Cancellations,
    ) -> Self {
        InstanceLaunchManager {
            engine,
            hub,
            processes,
            cancellations,
            reserved: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Prepare and spawn a fresh session of an instance off-thread. Instances may
    /// run several sessions at once, so this always launches — it does not refuse
    /// a running instance. `profile` overrides the active content profile for
    /// this launch; `reconcile` off skips the sync/mirror pass (sessions are
    /// already running, so the mirror is in use). Returns the launch job id.
    pub fn start(
        &self,
        instance_id: String,
        account: String,
        profile: String,
        reconcile: bool,
        id: String,
    ) -> Option<String> {
        let id = job_id(id, "instance-launch");
        let (session_id, seq) = self.reserve_session(&instance_id);

        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let processes = self.processes.clone();
        let reserved = self.reserved.clone();
        let cancellations = self.cancellations.clone();
        let job_id = id.clone();
        tracing::info!(job = %id, instance = %instance_id, session = %session_id, account = %account, "instance launch started");

        tokio::spawn(async move {
            let progress_hub = hub.clone();
            let progress_id = job_id.clone();
            let on_progress: Box<dyn Fn(&ProvisionProgress) + Send + Sync> =
                Box::new(coalesce_progress(move |p: &ProvisionProgress| {
                    progress_hub.publish(&topic_event(&InstanceLaunchProgressEvent {
                        id: progress_id.clone(),
                        progress: p.clone(),
                    }));
                }));

            let (cancel, _registered) = cancellations.register(&job_id);
            let running = engine::Job::new(on_progress.as_ref(), &cancel);
            let outcome = launch(
                &engine,
                &processes,
                &instance_id,
                &session_id,
                seq,
                &account,
                &profile,
                reconcile,
                &running,
            )
            .await;
            // The supervisor now owns the id (or the launch failed) — release it.
            reserved.lock().unwrap().remove(&session_id);
            match outcome {
                Ok(launched) => {
                    tracing::info!(
                        job = %job_id,
                        process = %launched.process_id,
                        pid = launched.pid,
                        warnings = launched.warnings.len(),
                        "instance launch done"
                    );
                    hub.publish(&topic_event(&InstanceLaunchDoneEvent {
                        id: job_id.clone(),
                        process_id: launched.process_id,
                        pid: launched.pid,
                        warnings: launched.warnings,
                    }));
                }
                Err(LaunchFailure::Cancelled) => {
                    tracing::info!(job = %job_id, instance = %instance_id, "instance launch cancelled");
                    hub.publish(&topic_event(&InstanceLaunchCancelledEvent {
                        id: job_id.clone(),
                    }));
                }
                Err(LaunchFailure::Failed(error)) => {
                    tracing::error!(job = %job_id, instance = %instance_id, %error, "instance launch failed");
                    hub.publish(&topic_event(&InstanceLaunchErrorEvent {
                        id: job_id.clone(),
                        error,
                    }));
                }
            }
        });
        Some(id)
    }

    /// Claim the next free session id for an instance under the reservation lock,
    /// counting both live sessions and ids already reserved but not yet spawned.
    fn reserve_session(&self, instance_id: &str) -> (String, u32) {
        let prefix = instance_session_prefix(instance_id);
        let mut reserved = self.reserved.lock().unwrap();
        let live_max = self
            .processes
            .list()
            .into_iter()
            .filter_map(|p| {
                p.id.strip_prefix(&prefix)
                    .and_then(|s| s.parse::<u32>().ok())
            })
            .max();
        let mut seq = live_max.map_or(1, |n| n + 1);
        while reserved.contains(&instance_session_id(instance_id, seq)) {
            seq += 1;
        }
        let session_id = instance_session_id(instance_id, seq);
        reserved.insert(session_id.clone());
        (session_id, seq)
    }
}

/// Materialise the instance, then hand the plan to the supervisor under the
/// session's own id and per-session log file.
#[allow(clippy::too_many_arguments)]
async fn launch(
    engine: &Engine,
    processes: &ProcessSupervisor,
    instance_id: &str,
    session_id: &str,
    seq: u32,
    account: &str,
    profile: &str,
    reconcile: bool,
    on_progress: &engine::Job<'_>,
) -> Result<Launched, LaunchFailure> {
    let prepared = engine
        .prepare_instance(instance_id, account, seq, profile, reconcile, on_progress)
        .await
        .map_err(LaunchFailure::from)?;

    let spec = ProcessSpec {
        id: session_id.to_string(),
        program: prepared.plan.program.to_string_lossy().into_owned(),
        args: prepared.plan.args,
        log: LogSource::File(prepared.log_file),
        cwd: Some(prepared.plan.cwd),
        env: BTreeMap::new(),
        restart: RestartPolicy::Never,
    };
    match processes.start(spec).await {
        Ok(info) => {
            if let Err(e) = engine.instances().mark_launched(instance_id) {
                tracing::warn!(instance = %instance_id, error = %e, "failed to stamp last-played");
            }
            Ok(Launched {
                process_id: info.id,
                pid: info.pid,
                warnings: prepared.warnings,
            })
        }
        Err(StartError::EmptyProgram | StartError::InvalidId) => {
            Err(LaunchFailure::Failed(ErrorInfo::Internal {
                detail: "invalid launch plan".to_string(),
            }))
        }
        Err(StartError::Spawn(e)) => Err(LaunchFailure::Failed(ErrorInfo::Internal {
            detail: format!("cannot spawn the game: {e}"),
        })),
    }
}

/// Why a launch did not produce a session. Cancellation is kept apart from
/// failure all the way out: nothing went wrong, so the job must not report an
/// error to a front-end that would show it as one.
enum LaunchFailure {
    Cancelled,
    Failed(ErrorInfo),
}

impl From<anyhow::Error> for LaunchFailure {
    fn from(error: anyhow::Error) -> Self {
        match engine::is_cancelled(&error) {
            true => LaunchFailure::Cancelled,
            false => LaunchFailure::Failed(crate::runtime::engine_error(error)),
        }
    }
}

/// A started session, with whatever the preparation could not do properly.
struct Launched {
    process_id: String,
    pid: u32,
    warnings: Vec<WarningInfo>,
}
