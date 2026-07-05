//! `hestia server …` — browse server flavors/versions, create fully provisioned
//! servers, and drive them (start/stop/status/logs) through the daemon.

use std::sync::Arc;

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use client::proto::server::ServerInfo;
use client::Client;

use crate::commands::mc;
use crate::ui::{self, ProvisionReporter, Spinner, View};

const EULA_URL: &str = "https://aka.ms/MinecraftEULA";

#[derive(Subcommand)]
pub enum ServerCmd {
    /// Pick a flavor, then list the game versions it offers
    Available {
        /// Flavor id (e.g. vanilla, fabric); prompts interactively when omitted
        flavor: Option<String>,
        #[arg(long, help = "List the available flavors and exit (no prompt)")]
        flavors: bool,
        #[arg(long, help = "Include snapshots and old versions")]
        all: bool,
    },
    /// Create a fully provisioned server (jar downloaded, java installed)
    Create {
        /// Flavor id (e.g. vanilla, fabric)
        flavor: String,
        /// Game version (e.g. 1.21.1)
        version: String,
        #[arg(long, help = "Pin a loader version (modloaders only; default latest)")]
        loader: Option<String>,
        #[arg(long, help = "Display name (defaults to <flavor>-<version>)")]
        name: Option<String>,
        #[arg(
            long,
            help = "Accept the Minecraft EULA (https://aka.ms/MinecraftEULA)"
        )]
        eula: bool,
    },
    /// Managed servers and their state
    List,
    /// Delete a server (its jar, world and all)
    Remove {
        /// Server name or id
        server: String,
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
    /// A server's record merged with its live process state
    Status {
        /// Server name or id
        server: String,
    },
    /// Captured server output
    Logs {
        /// Server name or id
        server: String,
        #[arg(long, help = "Only the last N lines")]
        tail: Option<usize>,
    },
}

pub async fn run(cmd: ServerCmd) -> Result<()> {
    let client = super::connect().await?;
    match cmd {
        ServerCmd::Available {
            flavor,
            flavors,
            all,
        } => {
            let available = {
                let _spinner = Spinner::start("fetching flavors");
                client.server().flavors().await?
            };
            if flavors {
                return mc::show_flavors(&available);
            }
            let flavor = mc::pick_flavor(available, flavor)?;
            let versions = {
                let _spinner = Spinner::start("fetching versions");
                client.server().versions(&flavor).await?
            };
            mc::show_versions(&flavor, versions, all)?;
        }
        ServerCmd::Create {
            flavor,
            version,
            loader,
            name,
            eula,
        } => create(&client, flavor, version, loader, name, eula).await?,
        ServerCmd::List => list(&client).await?,
        ServerCmd::Remove { server } => {
            {
                let _spinner = Spinner::start(format!("removing '{server}'"));
                client.server().remove(&server).await?;
            }
            ui::show(View::line(format!("server '{server}' removed")))?;
        }
        ServerCmd::Start { server } => {
            let started = {
                let _spinner = Spinner::start(format!("starting '{server}'"));
                client.server().start(&server).await?
            };
            ui::show(View::line(format!(
                "server '{server}' started (pid {})",
                started.pid
            )))?;
        }
        ServerCmd::Stop { server } => {
            {
                let _spinner = Spinner::start(format!("stopping '{server}'"));
                client.server().stop(&server).await?;
            }
            ui::show(View::line(format!("server '{server}' stopped")))?;
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
    }
    Ok(())
}

async fn create(
    client: &Client,
    flavor: String,
    version: String,
    loader: Option<String>,
    name: Option<String>,
    eula: bool,
) -> Result<()> {
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

async fn list(client: &Client) -> Result<()> {
    let servers = client.server().list().await?;
    if servers.is_empty() {
        return ui::show(View::note("no servers yet (hestia server create …)"));
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
    let mut rows = vec![
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
    ];
    if let Some(process) = &info.process {
        if process.state == client::proto::process::ProcessState::Running {
            rows.push(("pid", process.pid.to_string()));
        }
    }
    ui::show(View::detail(rows))
}

fn state_label(info: &ServerInfo) -> String {
    if !info.ready {
        return "provisioning".into();
    }
    mc::process_state_label(&info.process)
}
