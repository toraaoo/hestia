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
pub mod server;
mod wizard;

use anyhow::{Context, Result};
use client::Client;

use crate::ui::Spinner;

/// Connect to the daemon, auto-spawning it if it is not already running.
pub async fn connect() -> Result<Client> {
    let _spinner = Spinner::start("connecting to the daemon");
    Client::connect(true)
        .await
        .context("cannot reach the daemon")
}

/// Connect only if the daemon is already running (no auto-spawn).
pub async fn connect_running() -> Result<Client> {
    Client::connect(false)
        .await
        .context("the daemon is not running")
}
