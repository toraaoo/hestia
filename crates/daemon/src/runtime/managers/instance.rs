use std::collections::BTreeMap;
use std::sync::Arc;

use engine::Engine;
use proto::instance::{
    InstanceLaunchDoneEvent, InstanceLaunchErrorEvent, InstanceLaunchProgressEvent,
};
use proto::minecraft::ProvisionProgress;
use proto::process::{LogSource, ProcessSpec, RestartPolicy};

use super::job::{job_id, topic_event, InFlight};
use crate::runtime::{instance_process_id, EventHub, ProcessSupervisor, StartError};

pub struct InstanceLaunchManager {
    engine: Arc<Engine>,
    hub: Arc<EventHub>,
    processes: Arc<ProcessSupervisor>,
    active: InFlight<String>,
}

impl InstanceLaunchManager {
    pub fn new(engine: Arc<Engine>, hub: Arc<EventHub>, processes: Arc<ProcessSupervisor>) -> Self {
        InstanceLaunchManager {
            engine,
            hub,
            processes,
            active: InFlight::new(),
        }
    }

    /// Prepare and spawn an instance off-thread, one launch per instance at a
    /// time. Returns the job id, or `None` if that instance is already
    /// launching.
    pub fn start(&self, instance_id: String, account: String, id: String) -> Option<String> {
        let id = job_id(id, "instance-launch");
        let Some(claim) = self.active.claim(instance_id.clone()) else {
            tracing::debug!(instance = %instance_id, "instance launch already in flight");
            return None;
        };

        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let processes = self.processes.clone();
        let job_id = id.clone();
        tracing::info!(job = %id, instance = %instance_id, account = %account, "instance launch started");

        tokio::spawn(async move {
            let _claim = claim;
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
