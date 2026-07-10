//! Launching an instance and driving it through the supervisor: launch, stop,
//! restart, remove, and its captured output.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use client::{Client, ProcessEvent};

use super::entry;
use crate::ui::{self, ProvisionReporter, Spinner, View};

/// Launch `reference`, rendering preparation progress; shared with `hestia play`.
pub async fn launch(client: &Client, reference: &str, account: &str) -> Result<()> {
    let reporter = Arc::new(ProvisionReporter::new());
    let progress = reporter.clone();
    let result = client
        .instance()
        .launch(reference, account, move |p| progress.update(p))
        .await;
    reporter.finish();
    let (_, pid) = result?;
    ui::show(View::line(format!(
        "instance '{reference}' launched (pid {pid})"
    )))
}

pub(super) async fn stop(client: &Client, instance: &str) -> Result<()> {
    {
        let _spinner = Spinner::start(format!("stopping '{instance}'"));
        client.instance().stop(instance).await?;
    }
    ui::show(View::line(format!("instance '{instance}' stopped")))
}

pub(super) async fn restart(client: &Client, instance: &str, account: &str) -> Result<()> {
    {
        let _spinner = Spinner::start(format!("stopping '{instance}'"));
        client.instance().stop(instance).await?;
        wait_until_stopped(client, instance).await?;
    }
    launch(client, instance, account).await
}

pub(super) async fn remove(client: &Client, instance: &str) -> Result<()> {
    {
        let _spinner = Spinner::start(format!("removing '{instance}'"));
        client.instance().remove(instance).await?;
    }
    ui::show(View::line(format!("instance '{instance}' removed")))
}

pub(super) async fn logs(
    client: &Client,
    instance: &str,
    tail: Option<usize>,
    follow: bool,
) -> Result<()> {
    let lines = client.instance().logs(instance, tail).await?;
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
