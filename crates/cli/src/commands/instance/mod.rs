//! `hestia instance …` — manage and launch client instances. Creation walks
//! through flavor/version pickers when arguments are omitted; files materialise
//! on first launch.
//!
//! The grammar is entry-first: catalogue verbs (`create`, `list`, `versions`,
//! `flavors`) take no entry, while everything that acts on one instance reads
//! as `instance <name> <action>`. This module is the grammar and the dispatch;
//! each verb group lives beside it.

mod backup;
mod config;
mod create;
mod entry;
pub(crate) mod lifecycle;
mod update;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use client::proto::content::ContentKind;
use client::Client;

use crate::commands::content::{self, ContentCmd, EntryKind};
use crate::commands::mc;
use crate::ui::Spinner;

pub use backup::BackupCmd;
pub use lifecycle::launch;

#[derive(Subcommand)]
#[command(
    after_help = "Act on one instance with `hestia instance <name> <action>`, e.g.\n  \
        hestia instance modded launch\n  \
        hestia instance modded mod add sodium\n  \
        hestia instance modded config set memory 4G\nRun `hestia instance <name> --help` for every action."
)]
pub enum InstanceCmd {
    /// Create an instance (prompts for anything omitted; files download at first launch)
    Create {
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
        #[arg(long, help = "Set -Xms and -Xmx together (e.g. 4G, 2048M)")]
        memory: Option<String>,
    },
    /// Managed instances and their state
    #[command(visible_alias = "ls")]
    List,
    /// Game versions a flavor offers (prompts for the flavor when omitted)
    Versions {
        /// Flavor id (e.g. vanilla, fabric)
        flavor: Option<String>,
        #[arg(long, help = "Include snapshots and old versions")]
        all: bool,
    },
    /// The available flavors
    Flavors,
    /// Act on one instance: `hestia instance <name> <launch|stop|mod|…>`
    #[command(external_subcommand)]
    Entry(Vec<String>),
}

/// The per-instance grammar reached through `hestia instance <name> …`. The
/// name is captured once here so no action has to repeat it.
#[derive(Parser)]
#[command(no_binary_name = true, name = "hestia instance")]
struct InstanceEntry {
    /// Instance name or id
    name: String,
    #[command(subcommand)]
    action: InstanceAction,
}

#[derive(Subcommand)]
enum InstanceAction {
    /// Prepare (java, client jar, libraries, assets) and launch the instance
    Launch {
        #[arg(long, help = "Account name or uuid (default: the switched-to account)")]
        account: Option<String>,
        #[arg(short, long, help = "Return immediately instead of following the logs")]
        detach: bool,
    },
    /// Kill the running instance
    Stop,
    /// Stop the running instance and launch it again
    Restart {
        #[arg(long, help = "Account name or uuid (default: the switched-to account)")]
        account: Option<String>,
        #[arg(short, long, help = "Return immediately instead of following the logs")]
        detach: bool,
    },
    /// The instance's record and process state
    Info,
    /// Captured instance output
    Logs {
        #[arg(short = 'n', long = "tail", help = "Only the last N lines")]
        tail: Option<usize>,
        #[arg(short, long, help = "Keep streaming new output until Ctrl-C")]
        follow: bool,
    },
    /// Get, set, or list settings (memory, jvm-args)
    Config {
        #[command(subcommand)]
        cmd: mc::ConfigCmd,
    },
    /// Archive, restore, or manage the instance's backups
    Backup {
        #[command(subcommand)]
        cmd: BackupCmd,
    },
    /// Install, list, remove, or update the instance's mods
    Mod {
        #[command(subcommand)]
        cmd: ContentCmd,
    },
    /// Install, list, remove, or update the instance's resource packs
    Resourcepack {
        #[command(subcommand)]
        cmd: ContentCmd,
    },
    /// Install, list, remove, or update the instance's shaders
    Shader {
        #[command(subcommand)]
        cmd: ContentCmd,
    },
    /// Install, list, remove, or update the instance's datapacks (into a world)
    Datapack {
        #[command(subcommand)]
        cmd: ContentCmd,
    },
    /// Move the instance to another version (prompts for anything omitted)
    Update {
        /// Target game version (prompts when omitted)
        version: Option<String>,
        #[arg(
            short,
            long,
            help = "Pin a loader version (modloaders only; default latest)"
        )]
        loader: Option<String>,
        #[arg(
            long,
            help = "Allow moving to an older version (saves do not downgrade)"
        )]
        downgrade: bool,
    },
    /// Rename the instance (moves its directory; must be stopped)
    Rename {
        /// The new display name
        new_name: String,
    },
    /// Delete the instance (its saves and all)
    #[command(visible_alias = "rm")]
    Remove,
}

pub async fn run(cmd: InstanceCmd) -> Result<()> {
    match cmd {
        InstanceCmd::Entry(argv) => {
            let InstanceEntry { name, action } = match InstanceEntry::try_parse_from(argv) {
                Ok(parsed) => parsed,
                Err(err) => err.exit(),
            };
            let client = super::connect().await?;
            run_action(&client, name, action).await
        }
        catalogue => {
            let client = super::connect().await?;
            match catalogue {
                InstanceCmd::Create {
                    flavor,
                    version,
                    loader,
                    name,
                    memory,
                } => create::run(&client, flavor, version, loader, name, memory).await,
                InstanceCmd::List => entry::list(&client).await,
                InstanceCmd::Versions { flavor, all } => versions(&client, flavor, all).await,
                InstanceCmd::Flavors => flavors(&client).await,
                InstanceCmd::Entry(_) => unreachable!("handled above"),
            }
        }
    }
}

async fn run_action(client: &Client, name: String, action: InstanceAction) -> Result<()> {
    match action {
        InstanceAction::Launch { account, detach } => {
            launch(
                client,
                &name,
                account.as_deref().unwrap_or_default(),
                detach,
            )
            .await
        }
        InstanceAction::Stop => lifecycle::stop(client, &name).await,
        InstanceAction::Restart { account, detach } => {
            lifecycle::restart(
                client,
                &name,
                account.as_deref().unwrap_or_default(),
                detach,
            )
            .await
        }
        InstanceAction::Info => {
            let instances = client.instance().list().await?;
            let Some(info) = instances.iter().find(|i| i.id == name || i.name == name) else {
                bail!("no instance matches '{name}'");
            };
            entry::show_info(info)
        }
        InstanceAction::Logs { tail, follow } => lifecycle::logs(client, &name, tail, follow).await,
        InstanceAction::Config { cmd } => config::run(client, &name, cmd).await,
        InstanceAction::Backup { cmd } => backup::run(client, &name, cmd).await,
        InstanceAction::Mod { cmd } => {
            content::run_entry(client, EntryKind::Instance, ContentKind::Mod, &name, cmd).await
        }
        InstanceAction::Resourcepack { cmd } => {
            content::run_entry(
                client,
                EntryKind::Instance,
                ContentKind::ResourcePack,
                &name,
                cmd,
            )
            .await
        }
        InstanceAction::Shader { cmd } => {
            content::run_entry(client, EntryKind::Instance, ContentKind::Shader, &name, cmd).await
        }
        InstanceAction::Datapack { cmd } => {
            content::run_entry(
                client,
                EntryKind::Instance,
                ContentKind::DataPack,
                &name,
                cmd,
            )
            .await
        }
        InstanceAction::Update {
            version,
            loader,
            downgrade,
        } => update::run(client, name, version, loader, downgrade).await,
        InstanceAction::Rename { new_name } => lifecycle::rename(client, &name, &new_name).await,
        InstanceAction::Remove => lifecycle::remove(client, &name).await,
    }
}

async fn versions(client: &Client, flavor: Option<String>, all: bool) -> Result<()> {
    let flavors = {
        let _spinner = Spinner::start("fetching flavors");
        client.instance().flavors().await?
    };
    let flavor = mc::pick_flavor(flavors, flavor)?;
    let versions = {
        let _spinner = Spinner::start("fetching versions");
        client.instance().versions(&flavor).await?
    };
    mc::show_versions(&flavor, versions, all)
}

async fn flavors(client: &Client) -> Result<()> {
    let flavors = {
        let _spinner = Spinner::start("fetching flavors");
        client.instance().flavors().await?
    };
    mc::show_flavors(&flavors)
}
