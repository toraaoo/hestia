//! CLI command groups, each driving the daemon over the client SDK.

pub mod account;
pub mod cache;
pub mod config;
pub mod content;
pub mod daemon;
pub mod instance;
pub mod java;
pub mod lifecycle;
mod mc;
pub mod modpack;
pub mod node;
pub mod play;
pub mod process;
pub mod remote;
pub mod server;
pub mod sync;
pub mod update;
mod wizard;

use std::sync::OnceLock;

use anyhow::{bail, Context, Result};
use client::Client;

use crate::ui::Spinner;

/// The node `--node` selected, if any. Set once from the parsed arguments, read
/// by every `connect()`: a command written against the local daemon then drives
/// a remote one without knowing which it got.
static NODE: OnceLock<String> = OnceLock::new();

pub fn target_node(node: Option<String>) {
    if let Some(node) = node.filter(|n| !n.is_empty()) {
        let _ = NODE.set(node);
    }
}

/// Connect to a running daemon; never spawns it. With `--node`, connects to that
/// node over HTTP instead — the same facades, a different wire.
pub async fn connect() -> Result<Client> {
    if let Some(node) = NODE.get() {
        let _spinner = Spinner::start(format!("reaching '{node}'"));
        let (entry, client) =
            Client::to_node(node).with_context(|| format!("cannot reach node '{node}'"))?;
        // Probed once, so an unreachable host fails here rather than inside
        // whichever command happened to run first.
        client
            .app()
            .ping()
            .await
            .with_context(|| format!("node '{}' did not answer", entry.label))?;
        client::remote::Registry::open(None).mark_seen(&entry.id);
        return Ok(client);
    }
    let _spinner = Spinner::start("connecting to the daemon");
    Client::connect()
        .await
        .context("the daemon is not running — start it with `hestia daemon start`")
}

/// Refuse a command that only makes sense against the local machine.
pub fn refuse_remote(what: &str) -> Result<()> {
    match NODE.get() {
        Some(node) => bail!(
            "`hestia {what}` runs on this machine; drop --node (currently '{node}'). A remote \
             node serves servers only."
        ),
        None => Ok(()),
    }
}

/// Probe for a running daemon; callers that treat "not running" as normal
/// (status, stop) use this.
pub async fn connect_running() -> Result<Client> {
    Client::connect().await.context("the daemon is not running")
}

/// Explicitly start the daemon and connect.
pub async fn start() -> Result<Client> {
    let _spinner = Spinner::start("starting the daemon");
    Client::start().await.context("cannot start the daemon")
}

/// Run a daemon job, turning Ctrl-C into an explicit cancellation of it.
///
/// Ctrl-C used to kill only this process while the daemon ran the job to
/// completion — a JDK arriving minutes after the user stopped waiting for it.
/// The daemon deliberately never cancels a job because its client vanished (a
/// job outlives the client that started it, like every supervised workload), so
/// the client has to *ask*, and this is where a terminal interrupt is turned
/// into that request.
///
/// The job's own future is still awaited afterwards, so the command exits on
/// the daemon's `cancelled` event rather than guessing that the cancel took.
pub async fn cancellable<T>(
    client: &Client,
    id: &str,
    job: impl std::future::Future<Output = Result<T, client::IpcError>>,
) -> Result<T, client::IpcError> {
    tokio::pin!(job);
    loop {
        tokio::select! {
            outcome = &mut job => return outcome,
            _ = tokio::signal::ctrl_c() => {
                let _ = client.cancel_job(id).await;
            }
        }
    }
}
