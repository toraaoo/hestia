//! `hestia server …` — create fully provisioned servers and drive them through
//! the daemon. Creation walks through flavor/version pickers (and the EULA
//! confirm) when arguments are omitted.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use client::proto::process::ProcessState;
use client::proto::server::ServerInfo;
use client::Client;

use crate::commands::mc;
use crate::ui::{self, ProvisionReporter, Spinner, View};

const EULA_URL: &str = "https://aka.ms/MinecraftEULA";

#[derive(Subcommand)]
pub enum ServerCmd {
    /// Create a fully provisioned server (prompts for anything omitted)
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
        #[arg(
            long,
            help = "Accept the Minecraft EULA (https://aka.ms/MinecraftEULA)"
        )]
        eula: bool,
    },
    /// Managed servers and their state
    #[command(visible_alias = "ls")]
    List,
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
        ServerCmd::Create {
            flavor,
            version,
            loader,
            name,
            eula,
        } => create(&client, flavor, version, loader, name, eula).await?,
        ServerCmd::List => list(&client).await?,
        ServerCmd::Start { server } => start(&client, &server).await?,
        ServerCmd::Stop { server } => {
            {
                let _spinner = Spinner::start(format!("stopping '{server}'"));
                client.server().stop(&server).await?;
            }
            ui::show(View::line(format!("server '{server}' stopped")))?;
        }
        ServerCmd::Restart { server } => {
            {
                let _spinner = Spinner::start(format!("stopping '{server}'"));
                client.server().stop(&server).await?;
                wait_until_stopped(&client, &server).await?;
            }
            start(&client, &server).await?;
        }
        ServerCmd::Status { server } => {
            let info = client.server().status(&server).await?;
            show_status(&info)?;
        }
        ServerCmd::Logs { server, tail } => {
            let lines = client.server().logs(&server, tail).await?;
            if lines.is_empty() {
                return ui::show(View::note("no output captured (has it been started?)"));
            }
            for line in lines {
                ui::show(View::line(line.line))?;
            }
        }
        ServerCmd::Remove { server } => {
            {
                let _spinner = Spinner::start(format!("removing '{server}'"));
                client.server().remove(&server).await?;
            }
            ui::show(View::line(format!("server '{server}' removed")))?;
        }
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

async fn create(
    client: &Client,
    flavor: Option<String>,
    version: Option<String>,
    loader: Option<String>,
    name: Option<String>,
    eula: bool,
) -> Result<()> {
    let flavors = {
        let _spinner = Spinner::start("fetching flavors");
        client.server().flavors().await?
    };
    let flavor = mc::pick_flavor(flavors, flavor)?;
    let versions = {
        let _spinner = Spinner::start("fetching versions");
        client.server().versions(&flavor).await?
    };
    let version = mc::pick_version(versions, version)?;
    if !eula {
        confirm_eula()?;
    }

    let reporter = Arc::new(ProvisionReporter::new());
    let progress = reporter.clone();
    let result = client
        .server()
        .create(
            name.as_deref().unwrap_or_default(),
            &flavor,
            &version,
            loader,
            true,
            move |p| progress.update(p),
        )
        .await;
    reporter.finish();
    let server = result?;
    ui::show(View::line(format!("server '{}' created", server.name)))?;
    show_status(&server)
}

/// Interactive fallback for a missing `--eula`; errors when stdin is not a
/// terminal so scripts must pass the flag explicitly.
fn confirm_eula() -> Result<()> {
    let choice = ui::select(
        &format!("running a Minecraft server requires accepting the EULA ({EULA_URL})"),
        &["accept".to_string(), "decline".to_string()],
    )
    .context("pass --eula to accept the Minecraft EULA")?;
    if choice != 0 {
        bail!("the Minecraft EULA was not accepted");
    }
    Ok(())
}

async fn start(client: &Client, server: &str) -> Result<()> {
    let started = {
        let _spinner = Spinner::start(format!("starting '{server}'"));
        client.server().start(server).await?
    };
    ui::show(View::line(format!(
        "server '{server}' started (pid {})",
        started.pid
    )))
}

/// Poll until the server's process has exited, so a restart's `start` does not
/// race the old child.
async fn wait_until_stopped(client: &Client, server: &str) -> Result<()> {
    for _ in 0..30 {
        let info = client.server().status(server).await?;
        let running = info
            .process
            .is_some_and(|p| p.state == ProcessState::Running);
        if !running {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    bail!("server '{server}' did not stop in time");
}

async fn list(client: &Client) -> Result<()> {
    let servers = client.server().list().await?;
    if servers.is_empty() {
        return ui::show(View::note("no servers yet (hestia server create)"));
    }
    let rows = servers
        .iter()
        .map(|s| {
            vec![
                s.name.clone(),
                s.flavor.clone(),
                s.game_version.clone(),
                s.loader_version.clone().unwrap_or_else(|| "-".into()),
                state_label(s),
            ]
        })
        .collect();
    ui::show(View::table(
        "servers",
        ["NAME", "FLAVOR", "VERSION", "LOADER", "STATE"],
        rows,
    ))
}

fn show_status(info: &ServerInfo) -> Result<()> {
    ui::show(View::detail([
        ("name", info.name.clone()),
        ("id", info.id.clone()),
        ("flavor", info.flavor.clone()),
        ("version", info.game_version.clone()),
        (
            "loader",
            info.loader_version.clone().unwrap_or_else(|| "-".into()),
        ),
        ("java", info.java_major.to_string()),
        ("state", state_label(info)),
    ]))
}

fn state_label(info: &ServerInfo) -> String {
    if !info.ready {
        return "provisioning".into();
    }
    mc::process_state_label(&info.process)
}
