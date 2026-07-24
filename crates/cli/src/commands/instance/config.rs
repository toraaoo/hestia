//! `hestia instance config <instance> get|set|list` — the per-instance JVM
//! settings surface. Changes apply on the next launch.

use anyhow::{bail, Result};
use client::Client;

use crate::commands::mc;
use crate::ui::{self, View};

pub(super) async fn run(client: &Client, instance: &str, cmd: mc::ConfigCmd) -> Result<()> {
    match cmd {
        mc::ConfigCmd::Get { key } => match client.instance().config_get(instance, &key).await? {
            Some(value) => ui::show(View::line(value))?,
            None => bail!("'{key}' is not set"),
        },
        mc::ConfigCmd::Set { key, value } => {
            client.instance().config_set(instance, &key, &value).await?;
            ui::show(View::note("applies from the next launch"))?;
        }
        mc::ConfigCmd::List => {
            let entries = client.instance().config_list(instance).await?;
            let defaults = mc::jvm_defaults(client).await;
            mc::show_config_entries(format!("{instance} config"), entries, &defaults)?;
        }
    }
    Ok(())
}
