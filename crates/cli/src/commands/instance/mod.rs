//! `hestia instance …` — manage and launch client instances. Creation walks
//! through flavor/version pickers when arguments are omitted; files materialise
//! on first launch.
//!
//! This module is the grammar and the dispatch; each verb group lives beside it.

mod backup;
mod config;
mod create;
mod entry;
mod lifecycle;
mod update;

use anyhow::{bail, Result};
use clap::Subcommand;
use client::proto::content::ContentKind;

use crate::commands::content::{self, EntryKind};
use crate::commands::mc;
use crate::ui::Spinner;

pub use backup::BackupCmd;
pub use lifecycle::launch;

#[derive(Subcommand)]
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
    /// Move a stopped instance to another version (prompts for anything omitted)
    Update {
        /// Instance name or id (prompts when omitted)
        instance: Option<String>,
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
    /// Managed instances and their state
    #[command(visible_alias = "ls")]
    List,
    /// Archive, restore, or manage an instance's backups (prompts for anything omitted)
    Backup {
        #[command(subcommand)]
        cmd: BackupCmd,
    },
    /// Get, set, or list this instance's settings (memory, jvm-args)
    Config {
        /// Instance name or id
        instance: String,
        #[command(subcommand)]
        cmd: mc::ConfigCmd,
    },
    /// Install, list, remove, or update this instance's mods
    Mod {
        #[command(subcommand)]
        cmd: content::ContentCmd,
    },
    /// Install, list, remove, or update this instance's resource packs
    Resourcepack {
        #[command(subcommand)]
        cmd: content::ContentCmd,
    },
    /// Install, list, remove, or update this instance's shaders
    Shader {
        #[command(subcommand)]
        cmd: content::ContentCmd,
    },
    /// Prepare (java, client jar, libraries, assets) and launch an instance
    Launch {
        /// Instance name or id
        instance: String,
        #[arg(long, help = "Account name or uuid (default: the switched-to account)")]
        account: Option<String>,
    },
    /// Kill a running instance
    Stop {
        /// Instance name or id
        instance: String,
    },
    /// Stop a running instance and launch it again
    Restart {
        /// Instance name or id
        instance: String,
        #[arg(long, help = "Account name or uuid (default: the switched-to account)")]
        account: Option<String>,
    },
    /// An instance's record and process state
    Info {
        /// Instance name or id
        instance: String,
    },
    /// Captured instance output
    Logs {
        /// Instance name or id
        instance: String,
        #[arg(short = 'n', long = "tail", help = "Only the last N lines")]
        tail: Option<usize>,
        #[arg(short, long, help = "Keep streaming new output until Ctrl-C")]
        follow: bool,
    },
    /// Delete an instance (its saves and all)
    #[command(visible_alias = "rm")]
    Remove {
        /// Instance name or id
        instance: String,
    },
    /// Game versions a flavor offers (prompts for the flavor when omitted)
    Versions {
        /// Flavor id (e.g. vanilla, fabric)
        flavor: Option<String>,
        #[arg(long, help = "Include snapshots and old versions")]
        all: bool,
    },
    /// The available flavors
    Flavors,
}

pub async fn run(cmd: InstanceCmd) -> Result<()> {
    let client = super::connect().await?;
    match cmd {
        InstanceCmd::Create {
            flavor,
            version,
            loader,
            name,
            memory,
        } => create::run(&client, flavor, version, loader, name, memory).await?,
        InstanceCmd::Update {
            instance,
            version,
            loader,
            downgrade,
        } => update::run(&client, instance, version, loader, downgrade).await?,
        InstanceCmd::List => entry::list(&client).await?,
        InstanceCmd::Backup { cmd } => backup::run(&client, cmd).await?,
        InstanceCmd::Config { instance, cmd } => config::run(&client, &instance, cmd).await?,
        InstanceCmd::Mod { cmd } => {
            content::run_entry(&client, EntryKind::Instance, ContentKind::Mod, cmd).await?
        }
        InstanceCmd::Resourcepack { cmd } => {
            content::run_entry(&client, EntryKind::Instance, ContentKind::ResourcePack, cmd).await?
        }
        InstanceCmd::Shader { cmd } => {
            content::run_entry(&client, EntryKind::Instance, ContentKind::Shader, cmd).await?
        }
        InstanceCmd::Launch { instance, account } => {
            launch(&client, &instance, account.as_deref().unwrap_or_default()).await?
        }
        InstanceCmd::Stop { instance } => lifecycle::stop(&client, &instance).await?,
        InstanceCmd::Restart { instance, account } => {
            lifecycle::restart(&client, &instance, account.as_deref().unwrap_or_default()).await?
        }
        InstanceCmd::Info { instance } => {
            let instances = client.instance().list().await?;
            let Some(info) = instances
                .iter()
                .find(|i| i.id == instance || i.name == instance)
            else {
                bail!("no instance matches '{instance}'");
            };
            entry::show_info(info)?;
        }
        InstanceCmd::Logs {
            instance,
            tail,
            follow,
        } => lifecycle::logs(&client, &instance, tail, follow).await?,
        InstanceCmd::Remove { instance } => lifecycle::remove(&client, &instance).await?,
        InstanceCmd::Versions { flavor, all } => {
            let flavors = {
                let _spinner = Spinner::start("fetching flavors");
                client.instance().flavors().await?
            };
            let flavor = mc::pick_flavor(flavors, flavor)?;
            let versions = {
                let _spinner = Spinner::start("fetching versions");
                client.instance().versions(&flavor).await?
            };
            mc::show_versions(&flavor, versions, all)?;
        }
        InstanceCmd::Flavors => {
            let flavors = {
                let _spinner = Spinner::start("fetching flavors");
                client.instance().flavors().await?
            };
            mc::show_flavors(&flavors)?;
        }
    }
    Ok(())
}
