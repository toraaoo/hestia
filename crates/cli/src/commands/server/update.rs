//! `hestia server update` — move a server to another version. A running server
//! confirms a stop-update-start; a downgrade confirms separately.

use std::sync::Arc;

use anyhow::{bail, Context, Result};
use client::proto::server::ServerUpdateParams;
use client::Client;

use super::{entry, lifecycle};
use crate::commands::mc;
use crate::ui::{self, ProvisionReporter, Spinner, View};

pub(super) async fn run(
    client: &Client,
    server: Option<String>,
    version: Option<String>,
    loader: Option<String>,
    downgrade: bool,
    restart: bool,
) -> Result<()> {
    let info = entry::pick_server(client.server().list().await?, server)?;
    let was_running = entry::running_process(&info).is_some();
    if was_running && !restart {
        confirm_update_restart(&info.name)?;
    }
    let versions = {
        let _spinner = Spinner::start("fetching versions");
        client.server().versions(&info.flavor).await?
    };
    let version = mc::pick_version(versions.clone(), version)?;
    let is_downgrade =
        client::proto::minecraft::downgrade_between(&versions, &info.game_version, &version)
            == Some(true);
    if is_downgrade && !downgrade {
        mc::confirm_downgrade(&info.name, "a world", &info.game_version, &version)?;
    }

    if was_running {
        {
            let _spinner = Spinner::start(format!("stopping '{}'", info.name));
            client.server().stop(&info.id).await?;
            lifecycle::wait_until_stopped(client, &info.id).await?;
        }
        ui::show(View::note(format!(
            "server '{}' stopped for the update",
            info.name
        )))?;
    }

    let reporter = Arc::new(ProvisionReporter::new());
    let progress = reporter.clone();
    let params = ServerUpdateParams {
        server: info.id.clone(),
        version: version.clone(),
        loader_version: loader,
        allow_downgrade: downgrade || is_downgrade,
        id: String::new(),
    };
    let result = client
        .server()
        .update(params, move |p| progress.update(p))
        .await;
    reporter.finish();
    let server = result?;
    ui::show(View::line(format!(
        "server '{}' updated to {}",
        server.name, server.game_version
    )))?;
    entry::show_status(&server)?;
    if was_running {
        lifecycle::start(client, &server.id).await?;
    }
    Ok(())
}

/// Interactive fallback for a missing `--restart`; errors when stdin is not a
/// terminal so scripts must pass the flag explicitly.
fn confirm_update_restart(name: &str) -> Result<()> {
    let choice = ui::select(
        &format!("server '{name}' is running and must restart to update"),
        &[
            "stop, update, and start again".to_string(),
            "cancel".to_string(),
        ],
    )
    .context("pass --restart to update a running server")?;
    if choice != 0 {
        bail!("update cancelled");
    }
    Ok(())
}
