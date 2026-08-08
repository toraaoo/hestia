//! `hestia node …` — the machines this control plane knows about.
//!
//! Distinct from `hestia remote`, which runs *on* a node and mints the keys.
//! This runs on your own machine and records where the nodes are.

use anyhow::{bail, Result};
use clap::Subcommand;
use client::remote::Registry;
use client::Client;

use crate::commands::mc::age_label;
use crate::ui::{self, View};

#[derive(Subcommand)]
pub enum NodeCmd {
    /// Add a node by URL and the key minted on it
    Add {
        /// What to call it here, e.g. `prod`
        label: String,
        /// Its base URL, e.g. https://prod.example.com
        url: String,
        #[arg(
            long,
            help = "The hst_ key from `hestia remote key create` on that node"
        )]
        key: String,
    },
    /// Every node this control plane knows
    #[command(alias = "ls")]
    List,
    /// Replace a node's key
    Rekey {
        /// Node label or id
        #[arg(value_name = "NODE")]
        reference: String,
        #[arg(long)]
        key: String,
    },
    /// Forget a node and its key
    #[command(alias = "rm")]
    Remove {
        /// Node label or id
        #[arg(value_name = "NODE")]
        reference: String,
    },
}

pub async fn run(cmd: NodeCmd) -> Result<()> {
    match cmd {
        NodeCmd::Add { label, url, key } => add(label, url, key).await,
        NodeCmd::List => list().await,
        NodeCmd::Rekey { reference, key } => rekey(reference, key),
        NodeCmd::Remove { reference } => remove(reference),
    }
}

async fn add(label: String, url: String, key: String) -> Result<()> {
    let registry = Registry::open(None);
    let entry = registry.add(&label, &url, &key)?;

    // Reached before it is reported as added: a typo would otherwise only
    // surface on the first real command.
    let (_, client) = Client::to_node(&entry.label)?;
    match client.server().list().await {
        Ok(servers) => {
            registry.mark_seen(&entry.id);
            ui::show(View::line(format!(
                "node '{}' added — {} servers",
                entry.label,
                servers.len()
            )))
        }
        Err(e) => {
            registry.remove(&entry.id)?;
            bail!("'{}' did not answer, so it was not added: {e}", entry.label);
        }
    }
}

async fn list() -> Result<()> {
    let nodes = Registry::open(None).list();
    if nodes.is_empty() {
        return ui::show(View::note(
            "no nodes — add one with `hestia node add <label> <url> --key hst_…`",
        ));
    }
    let rows = nodes
        .into_iter()
        .map(|node| {
            vec![
                node.label,
                node.url,
                match node.last_seen_unix {
                    0 => "never".to_string(),
                    seen => age_label(seen),
                },
            ]
        })
        .collect();
    ui::show(View::table("Nodes", ["LABEL", "URL", "LAST SEEN"], rows))
}

fn rekey(reference: String, key: String) -> Result<()> {
    let entry = Registry::open(None).rekey(&reference, &key)?;
    ui::show(View::line(format!(
        "'{}' now uses the new key",
        entry.label
    )))
}

fn remove(reference: String) -> Result<()> {
    let entry = Registry::open(None).remove(&reference)?;
    ui::show(View::line(format!(
        "node '{}' forgotten, and its key with it",
        entry.label
    )))
}
