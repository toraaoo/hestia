//! Driving a provisioned server through the supervisor: start, stop, restart,
//! remove, and its captured output.

use std::time::Duration;

use anyhow::{bail, Context, Result};
use client::proto::process::ProcessState;
use client::{Client, ProcessEvent};

use super::entry;
use crate::ui::{self, MonitorSample, Spinner, View};

pub(crate) async fn start(client: &Client, server: &str) -> Result<()> {
    let pid = start_quiet(client, server).await?;
    ui::show(View::line(format!("server '{server}' started (pid {pid})")))
}

/// Start without the stdout line — the attach path prints its outcome only
/// after the console session ends, so nothing lands in the shell between the
/// prompt and the alternate screen (which some terminals duplicate into
/// scrollback).
pub(crate) async fn start_quiet(client: &Client, server: &str) -> Result<u32> {
    let started = {
        let _spinner = Spinner::start(format!("starting '{server}'"));
        client.server().start(server).await?
    };
    Ok(started.pid)
}

pub(crate) async fn stop(client: &Client, server: &str) -> Result<()> {
    {
        let _spinner = Spinner::start(format!("stopping '{server}'"));
        client.server().stop(server).await?;
    }
    ui::show(View::line(format!("server '{server}' stopped")))
}

pub(crate) async fn restart(client: &Client, server: &str) -> Result<()> {
    {
        let _spinner = Spinner::start(format!("stopping '{server}'"));
        client.server().stop(server).await?;
        wait_until_stopped(client, server).await?;
    }
    start(client, server).await
}

pub(super) async fn remove(client: &Client, server: &str) -> Result<()> {
    {
        let _spinner = Spinner::start(format!("removing '{server}'"));
        client.server().remove(server).await?;
    }
    ui::show(View::line(format!("server '{server}' removed")))
}

pub(crate) async fn rename(client: &Client, server: &str, new_name: &str) -> Result<()> {
    let info = {
        let _spinner = Spinner::start(format!("renaming '{server}'"));
        client.server().rename(server, new_name).await?
    };
    ui::show(View::line(format!(
        "server '{server}' renamed to '{}' (id {})",
        info.name, info.id
    )))
}

pub(crate) async fn logs(
    client: &Client,
    server: &str,
    tail: Option<usize>,
    follow: bool,
) -> Result<()> {
    let lines = client.server().logs(server, tail).await?;
    if follow && ui::interactive_output() {
        let info = client.server().status(server, false).await?;
        let process = entry::running_process(&info)
            .with_context(|| format!("server '{}' is not running", info.name))?;
        let backfill = lines.into_iter().map(|l| l.line).collect();
        return crate::commands::lifecycle::log_session(
            client,
            &info.name,
            &process.id,
            backfill,
            "server",
        )
        .await;
    }
    if lines.is_empty() && !follow {
        return ui::show(View::note("no output captured (has it been started?)"));
    }
    for line in lines {
        ui::show(View::line(line.line))?;
    }
    if follow {
        follow_logs(client, server).await?;
    }
    Ok(())
}

/// Run the fullscreen resource monitor over the running server's process,
/// filtering the daemon's metrics stream to it and feeding the graph.
pub(crate) async fn monitor(client: &Client, server: &str) -> Result<()> {
    let info = client.server().status(server, false).await?;
    let process = entry::running_process(&info)
        .with_context(|| format!("server '{}' is not running", info.name))?;
    let target = process.id;

    let mut samples = client.process().subscribe_metrics().await?;
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let forward = tokio::spawn(async move {
        while let Some(batch) = samples.recv().await {
            let sample = batch
                .into_iter()
                .find(|m| m.id == target)
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

async fn follow_logs(client: &Client, server: &str) -> Result<()> {
    let info = client.server().status(server, false).await?;
    let process = entry::running_process(&info)
        .with_context(|| format!("server '{}' is not running", info.name))?;
    let mut events = client.process().subscribe(&process.id).await?;
    while let Some(event) = events.recv().await {
        match event {
            ProcessEvent::Output(line) => ui::show(View::line(line.line))?,
            ProcessEvent::Exit(_) => {
                return ui::show(View::note("server stopped"));
            }
        }
    }
    Ok(())
}

/// Poll until the server's process reports running, so an attach right after
/// `start` does not race the spawn.
pub(crate) async fn wait_until_running(client: &Client, server: &str) -> Result<()> {
    for _ in 0..20 {
        let info = client.server().status(server, false).await?;
        let running = info
            .process
            .is_some_and(|p| p.state == ProcessState::Running);
        if running {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    bail!("server '{server}' did not report running in time");
}

/// Poll until the server's process has exited, so a restart's `start` does not
/// race the old child.
pub(super) async fn wait_until_stopped(client: &Client, server: &str) -> Result<()> {
    for _ in 0..30 {
        let info = client.server().status(server, false).await?;
        let running = info
            .process
            .is_some_and(|p| p.state == ProcessState::Running);
        if !running {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    bail!("server '{server}' did not stop in time");
}
