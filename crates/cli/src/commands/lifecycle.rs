//! Top-level lifecycle shortcuts: `hestia start|stop|restart|logs|rename <name>`.
//!
//! A server and an instance are driven the same way day to day, but they live
//! in separate registries with different verbs (`server start` vs `instance
//! launch`). These verb-first shortcuts resolve a name across both so the
//! common actions do not force the caller to first recall which kind a name is.

use anyhow::{bail, Result};
use client::{Client, ProcessEvent};

use super::{connect, instance, server};
use crate::ui::{self, ConsoleEvent, View};

enum Target {
    Server,
    Instance,
}

/// Resolve a name (or id) to the single server or instance it identifies,
/// erroring when it matches both or neither.
async fn resolve(client: &Client, name: &str) -> Result<Target> {
    let is_server = client
        .server()
        .list()
        .await?
        .iter()
        .any(|s| s.id == name || s.name == name);
    let is_instance = client
        .instance()
        .list()
        .await?
        .iter()
        .any(|i| i.id == name || i.name == name);
    match (is_server, is_instance) {
        (true, false) => Ok(Target::Server),
        (false, true) => Ok(Target::Instance),
        (true, true) => bail!(
            "'{name}' names both a server and an instance; \
             use `hestia server {name} …` or `hestia instance {name} …`"
        ),
        (false, false) => bail!("no server or instance matches '{name}'"),
    }
}

pub async fn start(name: String, account: Option<String>, detach: bool) -> Result<()> {
    let client = connect().await?;
    match resolve(&client, &name).await? {
        Target::Server => server::console::start_attached(client, &name, detach).await,
        Target::Instance => {
            instance::launch(
                &client,
                &name,
                account.as_deref().unwrap_or_default(),
                false,
                detach,
            )
            .await
        }
    }
}

pub async fn stop(name: String, session: Option<String>) -> Result<()> {
    let client = connect().await?;
    match resolve(&client, &name).await? {
        Target::Server => {
            reject_server_session(&session)?;
            server::lifecycle::stop(&client, &name).await
        }
        Target::Instance => instance::lifecycle::stop(&client, &name, session).await,
    }
}

pub async fn restart(
    name: String,
    session: Option<String>,
    account: Option<String>,
    detach: bool,
) -> Result<()> {
    let client = connect().await?;
    match resolve(&client, &name).await? {
        Target::Server => {
            reject_server_session(&session)?;
            server::console::restart_attached(client, &name, detach).await
        }
        Target::Instance => {
            instance::lifecycle::restart(
                &client,
                &name,
                session,
                account.as_deref().unwrap_or_default(),
                detach,
            )
            .await
        }
    }
}

pub async fn logs(
    name: String,
    session: Option<String>,
    tail: Option<usize>,
    follow: bool,
) -> Result<()> {
    let client = connect().await?;
    match resolve(&client, &name).await? {
        Target::Server => {
            reject_server_session(&session)?;
            server::lifecycle::logs(&client, &name, tail, follow).await
        }
        Target::Instance => instance::lifecycle::logs(&client, &name, session, tail, follow).await,
    }
}

/// A server runs a single process, so `--session` is meaningless for one.
fn reject_server_session(session: &Option<String>) -> Result<()> {
    if session.is_some() {
        bail!("--session applies to instances only; a server runs a single process");
    }
    Ok(())
}

pub async fn rename(name: String, new_name: String) -> Result<()> {
    let client = connect().await?;
    match resolve(&client, &name).await? {
        Target::Server => server::lifecycle::rename(&client, &name, &new_name).await,
        Target::Instance => instance::lifecycle::rename(&client, &name, &new_name).await,
    }
}

/// Run the read-only fullscreen log session over a running process: feed the
/// backfill, subscribe to its output, and stream until detach or exit. Prints
/// the plain outcome after the terminal is restored, so the shell keeps a
/// record.
pub(crate) async fn log_session(
    client: &Client,
    name: &str,
    process_id: &str,
    backfill: Vec<String>,
    noun: &str,
) -> Result<()> {
    let mut events = client.process().subscribe(process_id).await?;
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
    let forward = tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            let message = match event {
                ProcessEvent::Output(line) => ConsoleEvent::Output(line.line),
                ProcessEvent::Exit(_) => ConsoleEvent::Closed("stopped".to_string()),
            };
            if event_tx.send(message).is_err() {
                break;
            }
        }
    });
    let title = format!("{name} — logs");
    let closed =
        tokio::task::spawn_blocking(move || ui::log_session(&title, backfill, event_rx)).await??;
    forward.abort();
    match closed {
        Some(message) => ui::show(View::note(format!("{noun} '{name}' {message}"))),
        None => ui::show(View::note(format!("detached — '{name}' still running"))),
    }
}
