//! `hestia instance …` — manage and launch client instances. Creation walks
//! through flavor/version pickers when arguments are omitted; files materialise
//! on first launch.

use std::sync::Arc;

use anyhow::{bail, Result};
use clap::Subcommand;
use client::proto::instance::InstanceInfo;
use client::Client;

use crate::commands::mc;
use crate::ui::{self, ProvisionReporter, Spinner, View};

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
    },
    /// Managed instances and their state
    #[command(visible_alias = "ls")]
    List,
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
    /// An instance's record and process state
    Info {
        /// Instance name or id
        instance: String,
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
        } => create(&client, flavor, version, loader, name).await?,
        InstanceCmd::List => list(&client).await?,
        InstanceCmd::Launch { instance, account } => {
            launch(&client, &instance, account.as_deref().unwrap_or_default()).await?
        }
        InstanceCmd::Stop { instance } => {
            {
                let _spinner = Spinner::start(format!("stopping '{instance}'"));
                client.instance().stop(&instance).await?;
            }
            ui::show(View::line(format!("instance '{instance}' stopped")))?;
        }
        InstanceCmd::Info { instance } => {
            let instances = client.instance().list().await?;
            let Some(info) = instances
                .iter()
                .find(|i| i.id == instance || i.name == instance)
            else {
                bail!("no instance matches '{instance}'");
            };
            show_info(info)?;
        }
        InstanceCmd::Remove { instance } => {
            {
                let _spinner = Spinner::start(format!("removing '{instance}'"));
                client.instance().remove(&instance).await?;
            }
            ui::show(View::line(format!("instance '{instance}' removed")))?;
        }
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

/// Launch `reference`, rendering preparation progress; shared with `hestia play`.
pub async fn launch(client: &Client, reference: &str, account: &str) -> Result<()> {
    let reporter = Arc::new(ProvisionReporter::new());
    let progress = reporter.clone();
    let result = client
        .instance()
        .launch(reference, account, move |p| progress.update(p))
        .await;
    reporter.finish();
    let (_, pid) = result?;
    ui::show(View::line(format!(
        "instance '{reference}' launched (pid {pid})"
    )))
}

async fn create(
    client: &Client,
    flavor: Option<String>,
    version: Option<String>,
    loader: Option<String>,
    name: Option<String>,
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
    show_info(&instance)
}

async fn list(client: &Client) -> Result<()> {
    let instances = client.instance().list().await?;
    if instances.is_empty() {
        return ui::show(View::note("no instances yet (hestia instance create)"));
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

fn show_info(info: &InstanceInfo) -> Result<()> {
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
        ("state", mc::process_state_label(&info.process)),
    ]))
}
