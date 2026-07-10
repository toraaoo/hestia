//! `hestia server …` — create fully provisioned servers and drive them through
//! the daemon. Creation walks through flavor/version pickers (and the EULA
//! confirm) when arguments are omitted.
//!
//! This module is the grammar and the dispatch; each verb group lives beside it.

mod backup;
mod config;
mod console;
mod create;
mod entry;
mod lifecycle;
mod update;

use anyhow::Result;
use clap::Subcommand;
use client::proto::content::ContentKind;

use crate::commands::content::{self, EntryKind};
use crate::commands::mc;
use crate::ui::Spinner;

pub use backup::BackupCmd;
pub use create::CreateArgs;

#[derive(Subcommand)]
pub enum ServerCmd {
    /// Create a fully provisioned server (prompts for anything omitted)
    Create {
        #[command(flatten)]
        args: CreateArgs,
    },
    /// Move a server to another version (prompts for anything omitted)
    Update {
        /// Server name or id (prompts when omitted)
        server: Option<String>,
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
            help = "Allow moving to an older version (worlds do not downgrade)"
        )]
        downgrade: bool,
        #[arg(
            long,
            help = "Stop a running server for the update and start it again after"
        )]
        restart: bool,
    },
    /// Managed servers and their state
    #[command(visible_alias = "ls")]
    List,
    /// Archive, restore, or manage a server's backups (prompts for anything omitted)
    Backup {
        #[command(subcommand)]
        cmd: BackupCmd,
    },
    /// Get, set, or list this server's settings (memory, jvm-args,
    /// backup-interval, backup-retention, server.properties)
    Config {
        /// Server name or id
        server: String,
        #[command(subcommand)]
        cmd: mc::ConfigCmd,
    },
    /// Install, list, remove, or update this server's mods
    Mod {
        #[command(subcommand)]
        cmd: content::ContentCmd,
    },
    /// Attach an interactive console: live logs, type to send commands
    #[command(visible_alias = "console")]
    Attach {
        /// Server name or id
        server: String,
    },
    /// Send one console command and print the reply
    #[command(visible_alias = "cmd")]
    Command {
        /// Server name or id
        server: String,
        /// The command, as it would be typed in the console
        #[arg(required = true, trailing_var_arg = true)]
        command: Vec<String>,
    },
    /// Start a server under the daemon's supervisor
    Start {
        /// Server name or id
        server: String,
    },
    /// Stop a running server
    Stop {
        /// Server name or id
        server: String,
    },
    /// Stop a running server and start it again
    Restart {
        /// Server name or id
        server: String,
    },
    /// A server's record merged with its live process state
    Status {
        /// Server name or id
        server: String,
    },
    /// Captured server output
    Logs {
        /// Server name or id
        server: String,
        #[arg(short = 'n', long = "tail", help = "Only the last N lines")]
        tail: Option<usize>,
        #[arg(short, long, help = "Keep streaming new output until Ctrl-C")]
        follow: bool,
    },
    /// Delete a server (its jar, world and all)
    #[command(visible_alias = "rm")]
    Remove {
        /// Server name or id
        server: String,
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

pub async fn run(cmd: ServerCmd) -> Result<()> {
    let client = super::connect().await?;
    match cmd {
        ServerCmd::Create { args } => create::run(&client, args).await?,
        ServerCmd::Update {
            server,
            version,
            loader,
            downgrade,
            restart,
        } => update::run(&client, server, version, loader, downgrade, restart).await?,
        ServerCmd::List => entry::list(&client).await?,
        ServerCmd::Backup { cmd } => backup::run(&client, cmd).await?,
        ServerCmd::Config { server, cmd } => config::run(&client, &server, cmd).await?,
        ServerCmd::Mod { cmd } => {
            content::run_entry(&client, EntryKind::Server, ContentKind::Mod, cmd).await?
        }
        ServerCmd::Attach { server } => return console::attach(client, &server).await,
        ServerCmd::Command { server, command } => {
            let reply = client.server().command(&server, &command.join(" ")).await?;
            console::show_reply(&reply)?;
        }
        ServerCmd::Start { server } => lifecycle::start(&client, &server).await?,
        ServerCmd::Stop { server } => lifecycle::stop(&client, &server).await?,
        ServerCmd::Restart { server } => lifecycle::restart(&client, &server).await?,
        ServerCmd::Status { server } => {
            let info = client.server().status(&server).await?;
            entry::show_status(&info)?;
        }
        ServerCmd::Logs {
            server,
            tail,
            follow,
        } => lifecycle::logs(&client, &server, tail, follow).await?,
        ServerCmd::Remove { server } => lifecycle::remove(&client, &server).await?,
        ServerCmd::Versions { flavor, all } => {
            let flavors = {
                let _spinner = Spinner::start("fetching flavors");
                client.server().flavors().await?
            };
            let flavor = mc::pick_flavor(flavors, flavor)?;
            let versions = {
                let _spinner = Spinner::start("fetching versions");
                client.server().versions(&flavor).await?
            };
            mc::show_versions(&flavor, versions, all)?;
        }
        ServerCmd::Flavors => {
            let flavors = {
                let _spinner = Spinner::start("fetching flavors");
                client.server().flavors().await?
            };
            mc::show_flavors(&flavors)?;
        }
    }
    Ok(())
}
