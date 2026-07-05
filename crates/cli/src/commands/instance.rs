//! `hestia instance …` — browse client flavors/versions and resolve a client.

use anyhow::Result;
use clap::Subcommand;
use client::proto::minecraft::InstanceProfile;

use crate::commands::mc;
use crate::ui::{self, Spinner, View};

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
    /// Resolve a flavor + version into a client profile and print its URL
    Create {
        /// Flavor id (e.g. vanilla, fabric)
        flavor: String,
        /// Game version (e.g. 1.21.1)
        version: String,
        #[arg(long, help = "Pin a loader version (modloaders only; default latest)")]
        loader: Option<String>,
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
        } => {
            let profile = {
                let _spinner = Spinner::start("resolving");
                client.instance().resolve(&flavor, &version, loader).await?
            };
            show_profile(profile)?;
        }
    }
    Ok(())
}

fn show_profile(profile: InstanceProfile) -> Result<()> {
    ui::show(View::detail([
        ("flavor", profile.flavor),
        ("version", profile.game_version),
        (
            "loader",
            profile.loader_version.unwrap_or_else(|| "-".into()),
        ),
        ("java", profile.java_major.to_string()),
        ("main", profile.main_class),
        ("assets", profile.asset_index.id),
        ("libraries", profile.libraries.len().to_string()),
    ]))?;
    ui::show(View::line(profile.client.url))
}
