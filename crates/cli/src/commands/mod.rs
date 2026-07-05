//! CLI command groups. Each mirrors a C++ `Command` and drives the daemon over
//! the client SDK.

pub mod auth;
pub mod cache;
pub mod config;
pub mod daemon;
pub mod instance;
pub mod java;
mod mc;
pub mod server;

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
