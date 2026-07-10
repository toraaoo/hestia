//! `hestia server create` — the flavor/version pickers, the EULA confirm, and
//! the create-time settings.

use std::sync::Arc;

use anyhow::{bail, Context, Result};
use client::proto::minecraft::ConfigEntry;
use client::proto::server::ServerCreateParams;
use client::Client;

use super::entry;
use crate::commands::mc;
use crate::ui::{self, ProvisionReporter, Spinner, View};

const EULA_URL: &str = "https://aka.ms/MinecraftEULA";

/// Everything `server create` accepts; folded into the create params and the
/// create-time settings list.
#[derive(clap::Args)]
pub struct CreateArgs {
    /// Flavor id (e.g. vanilla, fabric)
    flavor: Option<String>,
    /// Game version (e.g. 1.21.1)
    version: Option<String>,
    #[arg(
        short,
        long,
        help = "Pin a loader version (modloaders only; default latest)"
    )]
    loader: Option<String>,
    #[arg(short, long, help = "Display name (defaults to <flavor>-<version>)")]
    name: Option<String>,
    #[arg(
        long,
        help = "Accept the Minecraft EULA (https://aka.ms/MinecraftEULA)"
    )]
    eula: bool,
    #[arg(short, long, help = "Pin the game port (default: lowest free)")]
    port: Option<u16>,
    #[arg(long, help = "Set -Xms and -Xmx together (e.g. 4G, 2048M)")]
    memory: Option<String>,
    #[arg(long, help = "The message of the day shown in the server list")]
    motd: Option<String>,
    #[arg(long, help = "Maximum number of players")]
    max_players: Option<u32>,
    #[arg(long, value_parser = ["peaceful", "easy", "normal", "hard"])]
    difficulty: Option<String>,
    #[arg(long, value_parser = ["survival", "creative", "adventure", "spectator"])]
    gamemode: Option<String>,
    #[arg(long, help = "World seed (level-seed)")]
    seed: Option<String>,
    #[arg(
        long = "prop",
        value_name = "KEY=VALUE",
        help = "Set any other server.properties entry (repeatable; wins over the dedicated flags)"
    )]
    prop: Vec<String>,
}

pub(super) async fn run(client: &Client, args: CreateArgs) -> Result<()> {
    let flavors = {
        let _spinner = Spinner::start("fetching flavors");
        client.server().flavors().await?
    };
    let flavor = mc::pick_flavor(flavors, args.flavor)?;
    let versions = {
        let _spinner = Spinner::start("fetching versions");
        client.server().versions(&flavor).await?
    };
    let version = mc::pick_version(versions, args.version)?;
    let name = match args.name {
        Some(name) => name,
        None => ui::input("server name", &format!("{flavor}-{version}"))?,
    };
    if !args.eula {
        confirm_eula()?;
    }
    let config = build_config(
        args.memory,
        [
            ("motd", args.motd),
            ("max-players", args.max_players.map(|n| n.to_string())),
            ("difficulty", args.difficulty),
            ("gamemode", args.gamemode),
            ("level-seed", args.seed),
        ],
        args.prop,
    )?;

    let reporter = Arc::new(ProvisionReporter::new());
    let progress = reporter.clone();
    let params = ServerCreateParams {
        name,
        flavor,
        version,
        loader_version: args.loader,
        eula: true,
        port: args.port,
        config,
        id: String::new(),
    };
    let result = client
        .server()
        .create(params, move |p| progress.update(p))
        .await;
    reporter.finish();
    let server = result?;
    ui::show(View::line(format!("server '{}' created", server.name)))?;
    entry::show_status(&server)
}

/// Fold the create-time settings into one entries list: `--memory`, then the
/// dedicated property flags, then `--prop KEY=VALUE` (split on the first `=`;
/// a missing `=` is an error). Entries apply in order, so a `--prop` naming
/// the same key as a dedicated flag wins.
fn build_config(
    memory: Option<String>,
    flags: impl IntoIterator<Item = (&'static str, Option<String>)>,
    prop: Vec<String>,
) -> Result<Vec<ConfigEntry>> {
    let mut config = Vec::new();
    if let Some(memory) = memory {
        config.push(ConfigEntry {
            key: "memory".into(),
            value: memory,
        });
    }
    for (key, value) in flags {
        if let Some(value) = value {
            config.push(ConfigEntry {
                key: key.into(),
                value,
            });
        }
    }
    for entry in prop {
        let (key, value) = entry
            .split_once('=')
            .with_context(|| format!("--prop '{entry}' must be KEY=VALUE"))?;
        config.push(ConfigEntry {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(config)
}

/// Interactive fallback for a missing `--eula`; errors when stdin is not a
/// terminal so scripts must pass the flag explicitly.
fn confirm_eula() -> Result<()> {
    let accepted = ui::confirm(
        &format!("running a Minecraft server requires accepting the EULA ({EULA_URL})"),
        "accept",
        "decline",
    )
    .context("pass --eula to accept the Minecraft EULA")?;
    if !accepted {
        bail!("the Minecraft EULA was not accepted");
    }
    Ok(())
}
