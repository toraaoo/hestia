//! Top-level lifecycle shortcuts: `hestia start|stop|restart|logs <name>`.
//!
//! A server and an instance are driven the same way day to day, but they live
//! in separate registries with different verbs (`server start` vs `instance
//! launch`). These verb-first shortcuts resolve a name across both so the
//! common actions do not force the caller to first recall which kind a name is.

use anyhow::{bail, Result};
use client::Client;

use super::{connect, instance, server};

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

pub async fn start(name: String, account: Option<String>) -> Result<()> {
    let client = connect().await?;
    match resolve(&client, &name).await? {
        Target::Server => server::lifecycle::start(&client, &name).await,
        Target::Instance => {
            instance::launch(&client, &name, account.as_deref().unwrap_or_default()).await
        }
    }
}

pub async fn stop(name: String) -> Result<()> {
    let client = connect().await?;
    match resolve(&client, &name).await? {
        Target::Server => server::lifecycle::stop(&client, &name).await,
        Target::Instance => instance::lifecycle::stop(&client, &name).await,
    }
}

pub async fn restart(name: String, account: Option<String>) -> Result<()> {
    let client = connect().await?;
    match resolve(&client, &name).await? {
        Target::Server => server::lifecycle::restart(&client, &name).await,
        Target::Instance => {
            instance::lifecycle::restart(&client, &name, account.as_deref().unwrap_or_default())
                .await
        }
    }
}

pub async fn logs(name: String, tail: Option<usize>, follow: bool) -> Result<()> {
    let client = connect().await?;
    match resolve(&client, &name).await? {
        Target::Server => server::lifecycle::logs(&client, &name, tail, follow).await,
        Target::Instance => instance::lifecycle::logs(&client, &name, tail, follow).await,
    }
}
