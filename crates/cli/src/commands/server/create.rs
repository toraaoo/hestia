//! `hestia server create` — the flavor/version pickers, the EULA confirm, and
//! the create-time settings.

use std::sync::Arc;

use anyhow::{bail, Context, Result};
use client::proto::minecraft::ConfigEntry;
use client::proto::server::ServerCreateParams;
use client::Client;

use super::entry;
use crate::commands::mc;
use crate::commands::wizard::{self, Field, WizardKind, WizardOutcome, WizardSeed};
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

impl CreateArgs {
    /// No argument or flag at all — the caller asked for the interactive
    /// wizard. Anything given selects the flag-driven path instead.
    fn is_bare(&self) -> bool {
        self.flavor.is_none()
            && self.version.is_none()
            && self.loader.is_none()
            && self.name.is_none()
            && !self.eula
            && self.port.is_none()
            && self.memory.is_none()
            && self.motd.is_none()
            && self.max_players.is_none()
            && self.difficulty.is_none()
            && self.gamemode.is_none()
            && self.seed.is_none()
            && self.prop.is_empty()
    }
}

pub(super) async fn run(client: &Client, args: CreateArgs) -> Result<()> {
    if args.is_bare() && ui::interactive_output() {
        return run_wizard(client, args).await;
    }
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

/// The interactive path: the fullscreen step wizard, prefilled from whatever
/// flags were given.
async fn run_wizard(client: &Client, args: CreateArgs) -> Result<()> {
    let flavors = {
        let _spinner = Spinner::start("fetching flavors");
        client.server().flavors().await?
    };
    if let Some(flavor) = &args.flavor {
        if !flavors.iter().any(|f| &f.id == flavor) {
            let ids: Vec<&str> = flavors.iter().map(|f| f.id.as_str()).collect();
            bail!("unknown flavor '{flavor}' (available: {})", ids.join(", "));
        }
    }
    let extra = build_config(None, [], args.prop)?;
    let seed = WizardSeed {
        kind: WizardKind::Server,
        flavors,
        flavor: args.flavor,
        version: args.version,
        name: args.name,
        loader: args.loader,
        eula: args.eula,
        fields: vec![
            Field::text("memory", "memory", "JVM default", args.memory),
            Field::number(
                "port",
                "port",
                "auto — lowest free from 25565",
                args.port.map(|p| p.to_string()),
            ),
            Field::text("motd", "motd", "server default", args.motd),
            Field::number(
                "max-players",
                "max players",
                "server default",
                args.max_players.map(|n| n.to_string()),
            ),
            Field::choice(
                "difficulty",
                "difficulty",
                "server default",
                &["peaceful", "easy", "normal", "hard"],
                args.difficulty,
            ),
            Field::choice(
                "gamemode",
                "gamemode",
                "server default",
                &["survival", "creative", "adventure", "spectator"],
                args.gamemode,
            ),
            Field::text("level-seed", "seed", "random", args.seed),
        ],
        extra,
    };
    match wizard::run(client, seed).await? {
        None => ui::show(View::note("cancelled")),
        Some(WizardOutcome::Server(server)) => {
            ui::show(View::line(format!("server '{}' created", server.name)))?;
            entry::show_status(&server)
        }
        Some(WizardOutcome::Instance(_)) => unreachable!("server wizard created an instance"),
    }
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
