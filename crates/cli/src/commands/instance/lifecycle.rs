//! Launching an instance and driving it through the supervisor: launch, stop,
//! restart, remove, and its captured output.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use client::proto::process::ProcessState;
use client::{Client, ProcessEvent};

use super::entry;
use crate::ui::{self, MonitorSample, ProvisionReporter, Spinner, View};

/// Launch `reference`, rendering preparation progress, then attach a
/// read-only log session (unless detached or piped); shared with
/// `hestia play`. Attaching prints its outcome only after the session ends,
/// so nothing lands in the shell between the prompt and the alternate screen
/// (which some terminals duplicate into scrollback).
pub async fn launch(
    client: &Client,
    reference: &str,
    account: &str,
    new_session: bool,
    detach: bool,
) -> Result<()> {
    let reporter = Arc::new(ProvisionReporter::new());
    let progress = reporter.clone();
    let result = client
        .instance()
        .launch(reference, account, new_session, "", move |p| {
            progress.update(p)
        })
        .await;
    reporter.finish();
    let (process_id, pid) = result?;
    if detach || !ui::interactive_output() {
        return ui::show(View::line(format!(
            "instance '{reference}' launched (pid {pid})"
        )));
    }
    let backfill = client
        .instance()
        .logs(reference, Some(process_id.clone()), Some(100))
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|l| l.line)
        .collect();
    crate::commands::lifecycle::log_session(client, reference, &process_id, backfill, "instance")
        .await
}

pub(crate) async fn stop(client: &Client, instance: &str, session: Option<String>) -> Result<()> {
    let target = resolve_session(client, instance, &session).await?;
    {
        let _spinner = Spinner::start(format!("stopping '{instance}'"));
        client.instance().stop(instance, target.clone()).await?;
    }
    match target {
        Some(id) => ui::show(View::line(format!(
            "instance '{instance}' session {} stopped",
            session_label(&id)
        ))),
        None => ui::show(View::line(format!("instance '{instance}' stopped"))),
    }
}

pub(crate) async fn restart(
    client: &Client,
    instance: &str,
    session: Option<String>,
    account: &str,
    detach: bool,
) -> Result<()> {
    let target = resolve_session(client, instance, &session).await?;
    {
        let _spinner = Spinner::start(format!("stopping '{instance}'"));
        client.instance().stop(instance, target.clone()).await?;
        match &target {
            Some(id) => wait_until_session_stopped(client, instance, id).await?,
            None => wait_until_stopped(client, instance).await?,
        }
    }
    // Restarting one session leaves the others running, so its relaunch must opt
    // into a concurrent session; a full restart stopped everything first.
    launch(client, instance, account, target.is_some(), detach).await
}

/// Resolve an optional `--session` handle to a full process id against the live
/// instance; `None` stays `None` (all sessions / newest).
async fn resolve_session(
    client: &Client,
    instance: &str,
    session: &Option<String>,
) -> Result<Option<String>> {
    match session {
        Some(input) => {
            let info = entry::fetch(client, instance).await?;
            Ok(Some(entry::resolve_session(&info, input)?))
        }
        None => Ok(None),
    }
}

fn session_label(process_id: &str) -> &str {
    process_id.rsplit('_').next().unwrap_or(process_id)
}

pub(super) async fn remove(client: &Client, instance: &str) -> Result<()> {
    {
        let _spinner = Spinner::start(format!("removing '{instance}'"));
        client.instance().remove(instance).await?;
    }
    ui::show(View::line(format!("instance '{instance}' removed")))
}

pub(crate) async fn rename(client: &Client, instance: &str, new_name: &str) -> Result<()> {
    let info = {
        let _spinner = Spinner::start(format!("renaming '{instance}'"));
        client.instance().rename(instance, new_name).await?
    };
    ui::show(View::line(format!(
        "instance '{instance}' renamed to '{}' (id {})",
        info.name, info.id
    )))
}

pub(crate) async fn logs(
    client: &Client,
    instance: &str,
    session: Option<String>,
    tail: Option<usize>,
    follow: bool,
) -> Result<()> {
    let target = resolve_session(client, instance, &session).await?;
    let lines = client
        .instance()
        .logs(instance, target.clone(), tail)
        .await?;
    if follow && ui::interactive_output() {
        let info = entry::fetch(client, instance).await?;
        let process_id = follow_target(&info, &target)?;
        let backfill = lines.into_iter().map(|l| l.line).collect();
        return crate::commands::lifecycle::log_session(
            client,
            &info.name,
            &process_id,
            backfill,
            "instance",
        )
        .await;
    }
    if lines.is_empty() && !follow {
        return ui::show(View::note("no output captured (has it been launched?)"));
    }
    for line in lines {
        ui::show(View::line(line.line))?;
    }
    if follow {
        let info = entry::fetch(client, instance).await?;
        follow_logs(client, &info, &target).await?;
    }
    Ok(())
}

/// Run the fullscreen resource monitor over one running session (named, else
/// the newest), filtering the daemon's metrics stream to it.
pub(crate) async fn monitor(
    client: &Client,
    instance: &str,
    session: Option<String>,
) -> Result<()> {
    let target = resolve_session(client, instance, &session).await?;
    let info = entry::fetch(client, instance).await?;
    let process_id = follow_target(&info, &target)?;

    let mut samples = client.process().subscribe_metrics().await?;
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let forward = tokio::spawn(async move {
        while let Some(batch) = samples.recv().await {
            let sample = batch
                .into_iter()
                .find(|m| m.id == process_id)
                .map(|m| MonitorSample {
                    cpu_pct: m.cpu_pct,
                    mem_bytes: m.mem_bytes,
                });
            if tx.send(sample).is_err() {
                break;
            }
        }
    });

    let title = format!("{} — resources", info.name);
    let result = tokio::task::spawn_blocking(move || ui::monitor(&title, rx)).await?;
    forward.abort();
    result
}

/// The process id to follow: the named session, else the newest running one.
fn follow_target(
    info: &client::proto::instance::InstanceInfo,
    target: &Option<String>,
) -> Result<String> {
    match target {
        Some(id) => Ok(id.clone()),
        None => entry::running_process(info)
            .map(|p| p.id)
            .with_context(|| format!("instance '{}' is not running", info.name)),
    }
}

async fn follow_logs(
    client: &Client,
    info: &client::proto::instance::InstanceInfo,
    target: &Option<String>,
) -> Result<()> {
    let process_id = follow_target(info, target)?;
    let mut events = client.process().subscribe(&process_id).await?;
    while let Some(event) = events.recv().await {
        match event {
            ProcessEvent::Output(line) => ui::show(View::line(line.line))?,
            ProcessEvent::Exit(_) => {
                return ui::show(View::note("instance stopped"));
            }
        }
    }
    Ok(())
}

/// Poll until no session of the instance is running, so a restart's `launch`
/// does not race the old game.
async fn wait_until_stopped(client: &Client, instance: &str) -> Result<()> {
    for _ in 0..30 {
        let instances = client.instance().list().await?;
        let running = instances
            .iter()
            .filter(|i| client::proto::naming::reference_matches(instance, &i.id, &i.name))
            .any(|i| entry::running_process(i).is_some());
        if !running {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    bail!("instance '{instance}' did not stop in time");
}

/// Poll until one specific session has exited (its siblings keep running).
async fn wait_until_session_stopped(
    client: &Client,
    instance: &str,
    session_id: &str,
) -> Result<()> {
    for _ in 0..30 {
        let info = entry::fetch(client, instance).await?;
        let still_running = info
            .sessions
            .iter()
            .any(|s| s.id == session_id && s.state == ProcessState::Running);
        if !still_running {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    bail!("instance '{instance}' session did not stop in time");
}
