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
        .any(|s| client::proto::naming::reference_matches(name, &s.id, &s.name));
    let is_instance = client
        .instance()
        .list()
        .await?
        .iter()
        .any(|i| client::proto::naming::reference_matches(name, &i.id, &i.name));
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

/// Run the read-only fullscreen log session over one process: feed the
/// backfill, subscribe to its output, and stream until detach or exit. Prints
/// the plain outcome after the terminal is restored, so the shell keeps a
/// record. For the attach that follows a launch — the process is the subject,
/// so its exit ends the session.
pub(crate) async fn log_session(
    client: &Client,
    name: &str,
    process_id: &str,
    backfill: Vec<String>,
    noun: &str,
) -> Result<()> {
    run_log_session(client, name, process_id, backfill, noun, Scope::Process).await
}

/// Run the log session over an *entry* (`server-<id>` / `instance-<id>`): the
/// subject is the server or instance, not the process it currently runs, so a
/// stop leaves the session open and the next start resumes the stream. Startable
/// against a stopped entry — the backfill is read from disk and the stream waits.
pub(crate) async fn entry_log_session(
    client: &Client,
    name: &str,
    entry_key: &str,
    backfill: Vec<String>,
    noun: &str,
) -> Result<()> {
    run_log_session(client, name, entry_key, backfill, noun, Scope::Entry).await
}

/// What a log session follows — the difference is only what a process exit
/// means: the end of the subject, or one run of it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Process,
    Entry,
}

async fn run_log_session(
    client: &Client,
    name: &str,
    key: &str,
    backfill: Vec<String>,
    noun: &str,
    scope: Scope,
) -> Result<()> {
    let mut events = client.process().subscribe(key).await?;
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
    let subject = format!("{noun} '{name}'");
    let stopped = subject.clone();
    let forward = tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            let message = match (event, scope) {
                (ProcessEvent::Output(line), _) => ConsoleEvent::Output(line.line),
                (ProcessEvent::Exit(_), Scope::Process) => {
                    ConsoleEvent::Closed(format!("{stopped} stopped"))
                }
                (ProcessEvent::Exit(_), Scope::Entry) => {
                    ConsoleEvent::Notice(format!("— {stopped} stopped —"))
                }
                (ProcessEvent::Started(e), Scope::Entry) => {
                    ConsoleEvent::Notice(format!("— {stopped} started (pid {}) —", e.pid))
                }
                (ProcessEvent::Started(_), Scope::Process) => continue,
            };
            if event_tx.send(message).is_err() {
                return;
            }
        }
        // The stream ends only with the connection: say so rather than leaving a
        // silent screen that looks like an idle workload.
        let _ = event_tx.send(ConsoleEvent::Closed(
            "connection to the daemon lost".to_string(),
        ));
    });
    let title = format!("{name} — logs");
    let closed =
        tokio::task::spawn_blocking(move || ui::log_session(&title, backfill, event_rx)).await??;
    forward.abort();
    match (closed, scope) {
        (Some(message), _) => ui::show(View::note(message)),
        (None, Scope::Process) => {
            ui::show(View::note(format!("detached — '{name}' still running")))
        }
        (None, Scope::Entry) => ui::show(View::note(format!("stopped following {subject}"))),
    }
}
