//! Launching an instance and driving it through the supervisor: launch, stop,
//! restart, remove, and its captured output.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use client::{Client, ProcessEvent};

use super::entry;
use crate::ui::{self, ProvisionReporter, Spinner, View};

/// Launch `reference`, rendering preparation progress, then attach a
/// read-only log session (unless detached or piped); shared with
/// `hestia play`. Attaching prints its outcome only after the session ends,
/// so nothing lands in the shell between the prompt and the alternate screen
/// (which some terminals duplicate into scrollback).
pub async fn launch(client: &Client, reference: &str, account: &str, detach: bool) -> Result<()> {
    let reporter = Arc::new(ProvisionReporter::new());
    let progress = reporter.clone();
    let result = client
        .instance()
        .launch(reference, account, move |p| progress.update(p))
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

pub(crate) async fn stop(client: &Client, instance: &str) -> Result<()> {
    {
        let _spinner = Spinner::start(format!("stopping '{instance}'"));
        client.instance().stop(instance, None).await?;
    }
    ui::show(View::line(format!("instance '{instance}' stopped")))
}

pub(crate) async fn restart(
    client: &Client,
    instance: &str,
    account: &str,
    detach: bool,
) -> Result<()> {
    {
        let _spinner = Spinner::start(format!("stopping '{instance}'"));
        client.instance().stop(instance, None).await?;
        wait_until_stopped(client, instance).await?;
    }
    launch(client, instance, account, detach).await
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
    tail: Option<usize>,
    follow: bool,
) -> Result<()> {
    let lines = client.instance().logs(instance, None, tail).await?;
    if follow && ui::interactive_output() {
        let instances = client.instance().list().await?;
        let info = instances
            .iter()
            .find(|i| i.id == instance || i.name == instance)
            .with_context(|| format!("no instance matches '{instance}'"))?;
        let process = entry::running_process(info)
            .with_context(|| format!("instance '{}' is not running", info.name))?;
        let backfill = lines.into_iter().map(|l| l.line).collect();
        return crate::commands::lifecycle::log_session(
            client,
            &info.name,
            &process.id,
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
        follow_logs(client, instance).await?;
    }
    Ok(())
}

async fn follow_logs(client: &Client, instance: &str) -> Result<()> {
    let instances = client.instance().list().await?;
    let info = instances
        .iter()
        .find(|i| i.id == instance || i.name == instance)
        .with_context(|| format!("no instance matches '{instance}'"))?;
    let process = entry::running_process(info)
        .with_context(|| format!("instance '{}' is not running", info.name))?;
    let mut events = client.process().subscribe(&process.id).await?;
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

/// Poll until the instance's process has exited, so a restart's `launch` does
/// not race the old game.
async fn wait_until_stopped(client: &Client, instance: &str) -> Result<()> {
    for _ in 0..30 {
        let instances = client.instance().list().await?;
        let running = instances
            .iter()
            .filter(|i| i.id == instance || i.name == instance)
            .any(|i| entry::running_process(i).is_some());
        if !running {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    bail!("instance '{instance}' did not stop in time");
}
