//! `hestia instance create` — the flavor/version pickers and the create-time
//! JVM settings. The record is cheap; files materialise at the first launch.

use anyhow::Result;
use client::proto::minecraft::ConfigEntry;
use client::Client;

use super::entry;
use crate::commands::mc;
use crate::ui::{self, Spinner, View};

pub(super) async fn run(
    client: &Client,
    flavor: Option<String>,
    version: Option<String>,
    loader: Option<String>,
    name: Option<String>,
    memory: Option<String>,
) -> Result<()> {
    let flavors = {
        let _spinner = Spinner::start("fetching flavors");
        client.instance().flavors().await?
    };
    let flavor = mc::pick_flavor(flavors, flavor)?;
    let versions = {
        let _spinner = Spinner::start("fetching versions");
        client.instance().versions(&flavor).await?
    };
    let version = mc::pick_version(versions, version)?;
    let name = match name {
        Some(name) => name,
        None => ui::input("instance name", &format!("{flavor}-{version}"))?,
    };
    let config = memory
        .map(|memory| ConfigEntry {
            key: "memory".into(),
            value: memory,
        })
        .into_iter()
        .collect();

    let instance = {
        let _spinner = Spinner::start("resolving profile");
        client
            .instance()
            .create(&name, &flavor, &version, loader, config)
            .await?
    };
    ui::show(View::line(format!("instance '{}' created", instance.name)))?;
    entry::show_info(&instance)
}
