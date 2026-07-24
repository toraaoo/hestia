//! `hestia server config <server> get|set|list` — the per-server settings
//! surface. Changes apply on the next start.

use anyhow::{bail, Result};
use client::Client;

use crate::commands::mc;
use crate::ui::{self, View};

pub(super) async fn run(client: &Client, server: &str, cmd: mc::ConfigCmd) -> Result<()> {
    match cmd {
        mc::ConfigCmd::Get { key } => match client.server().config_get(server, &key).await? {
            Some(value) => ui::show(View::line(value))?,
            None => bail!("'{key}' is not set"),
        },
        mc::ConfigCmd::Set { key, value } => {
            client.server().config_set(server, &key, &value).await?;
            ui::show(View::note("applies from the next start"))?;
        }
        mc::ConfigCmd::List => {
            let entries = client.server().config_list(server).await?;
            let defaults = mc::jvm_defaults(client).await;
            mc::show_config_entries(format!("{server} config"), entries, &defaults)?;
        }
    }
    Ok(())
}
