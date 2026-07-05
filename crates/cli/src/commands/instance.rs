//! `hestia instance …` — browse client flavors/versions, manage instances, and
//! launch them (files materialise on first launch).

use std::sync::Arc;

use anyhow::Result;
use clap::Subcommand;
use client::proto::instance::InstanceInfo;
use client::Client;

use crate::commands::mc;
use crate::ui::{self, ProvisionReporter, Spinner, View};

#[derive(Subcommand)]
pub enum InstanceCmd {
    /// Pick a flavor, then list the game versions it offers
    Available {
        /// Flavor id (e.g. vanilla, fabric); prompts interactively when omitted
        flavor: Option<String>,
        #[arg(long, help = "List the available flavors and exit (no prompt)")]
        flavors: bool,
        #[arg(long, help = "Include snapshots and old versions")]
        all: bool,
    },
    /// Create an instance (its files download at first launch)
    Create {
        /// Flavor id (e.g. vanilla, fabric)
        flavor: String,
        /// Game version (e.g. 1.21.1)
        version: String,
        #[arg(long, help = "Pin a loader version (modloaders only; default latest)")]
        loader: Option<String>,
        #[arg(long, help = "Display name (defaults to <flavor>-<version>)")]
        name: Option<String>,
    },
    /// Managed instances and their state
    List,
    /// Delete an instance (its saves and all)
    Remove {
        /// Instance name or id
        instance: String,
    },
    /// Prepare (java, client jar, libraries, assets) and launch an instance
    Launch {
        /// Instance name or id
        instance: String,
        #[arg(long, help = "Account name or uuid (default: the signed-in account)")]
        account: Option<String>,
    },
    /// Kill a running instance
    Stop {
        /// Instance name or id
        instance: String,
    },
}

pub async fn run(cmd: InstanceCmd) -> Result<()> {
    let client = super::connect().await?;
    match cmd {
        InstanceCmd::Available {
            flavor,
            flavors,
            all,
        } => {
            let available = {
                let _spinner = Spinner::start("fetching flavors");
                client.instance().flavors().await?
            };
            if flavors {
                return mc::show_flavors(&available);
            }
            let flavor = mc::pick_flavor(available, flavor)?;
            let versions = {
                let _spinner = Spinner::start("fetching versions");
                client.instance().versions(&flavor).await?
            };
            mc::show_versions(&flavor, versions, all)?;
        }
        InstanceCmd::Create {
            flavor,
            version,
            loader,
            name,
        } => {
            let instance = {
                let _spinner = Spinner::start("resolving profile");
                client
                    .instance()
                    .create(
                        name.as_deref().unwrap_or_default(),
                        &flavor,
                        &version,
                        loader,
                    )
                    .await?
            };
            ui::show(View::line(format!("instance '{}' created", instance.name)))?;
            show_status(&instance)?;
        }
        InstanceCmd::List => list(&client).await?,
        InstanceCmd::Remove { instance } => {
            {
                let _spinner = Spinner::start(format!("removing '{instance}'"));
                client.instance().remove(&instance).await?;
            }
            ui::show(View::line(format!("instance '{instance}' removed")))?;
        }
        InstanceCmd::Launch { instance, account } => {
            let reporter = Arc::new(ProvisionReporter::new());
            let progress = reporter.clone();
            let result = client
                .instance()
                .launch(
                    &instance,
                    account.as_deref().unwrap_or_default(),
                    move |p| progress.update(p),
                )
                .await;
            reporter.finish();
            let (_, pid) = result?;
            ui::show(View::line(format!(
                "instance '{instance}' launched (pid {pid})"
            )))?;
        }
        InstanceCmd::Stop { instance } => {
            {
                let _spinner = Spinner::start(format!("stopping '{instance}'"));
                client.instance().stop(&instance).await?;
            }
            ui::show(View::line(format!("instance '{instance}' stopped")))?;
        }
    }
    Ok(())
}

async fn list(client: &Client) -> Result<()> {
    let instances = client.instance().list().await?;
    if instances.is_empty() {
        return ui::show(View::note("no instances yet (hestia instance create …)"));
    }
    let rows = instances
        .iter()
        .map(|i| {
            vec![
                i.name.clone(),
                i.flavor.clone(),
                i.game_version.clone(),
                i.loader_version.clone().unwrap_or_else(|| "-".into()),
                mc::process_state_label(&i.process),
            ]
        })
        .collect();
    ui::show(View::table(
        "instances",
        ["NAME", "FLAVOR", "VERSION", "LOADER", "STATE"],
        rows,
    ))
}

fn show_status(info: &InstanceInfo) -> Result<()> {
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
    ]))
}
