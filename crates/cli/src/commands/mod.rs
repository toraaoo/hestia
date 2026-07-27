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
pub mod play;
pub mod process;
pub mod server;
pub mod sync;
mod wizard;

use anyhow::{Context, Result};
use client::Client;

use crate::ui::Spinner;

/// Connect to a running daemon; never spawns it.
pub async fn connect() -> Result<Client> {
    let _spinner = Spinner::start("connecting to the daemon");
    Client::connect()
        .await
        .context("the daemon is not running — start it with `hestia daemon start`")
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
