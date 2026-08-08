//! `hestia remote …` — the node's HTTP door and the keys that open it. Every
//! command here goes over the unix socket, which is the point: a key is minted
//! by someone already on the node.

use anyhow::{bail, Result};
use clap::Subcommand;
use client::proto::remote::{RemoteKey, Scope};

use crate::commands::mc::age_label;
use crate::ui::{self, View};

#[derive(Subcommand)]
pub enum RemoteCmd {
    /// Whether the HTTP door is open, and where
    #[command(alias = "info")]
    Status,
    /// API keys for this node
    Key {
        #[command(subcommand)]
        cmd: KeyCmd,
    },
}

#[derive(Subcommand)]
pub enum KeyCmd {
    /// Mint a key — shown once, and never recoverable
    Create {
        /// What this key is for, e.g. the machine that will hold it
        #[arg(long)]
        name: String,
        /// What it may do: server:read, server:control, server:write,
        /// server:backup, server:create, server:delete (repeatable, or comma-separated)
        #[arg(long, value_delimiter = ',', required = true)]
        scope: Vec<String>,
        /// Narrow it to these servers by name or id; omit for the whole node
        #[arg(long, value_delimiter = ',')]
        server: Vec<String>,
    },
    /// Every key this node holds
    #[command(alias = "ls")]
    List,
    /// Revoke a key by its id or the prefix `list` shows
    #[command(alias = "rm")]
    Revoke {
        /// The key id or its `hst_…` prefix
        key: String,
    },
}

pub async fn run(cmd: RemoteCmd) -> Result<()> {
    match cmd {
        RemoteCmd::Status => status().await,
        RemoteCmd::Key { cmd } => match cmd {
            KeyCmd::Create {
                name,
                scope,
                server,
            } => create(name, scope, server).await,
            KeyCmd::List => list().await,
            KeyCmd::Revoke { key } => revoke(key).await,
        },
    }
}

async fn status() -> Result<()> {
    let client = super::connect().await?;
    let status = client.remote().status().await?;
    ui::show(View::detail([
        (
            "door",
            match (status.enabled, status.address.is_empty()) {
                (false, _) => "closed (config set remote.enabled true)".to_string(),
                (true, true) => "asked for, not open".to_string(),
                (true, false) => format!("open on http://{}", status.address),
            },
        ),
        (
            "reachable",
            match status.exposed {
                true => "from off this machine".to_string(),
                false => "from this machine only".to_string(),
            },
        ),
        ("api", status.versions.join(", ")),
        ("keys", status.keys.to_string()),
    ]))?;
    if !status.refusal.is_empty() {
        return ui::show(View::warning(status.refusal));
    }
    if status.enabled && status.exposed {
        return ui::show(View::warning(
            "this door answers the network in plaintext — put a reverse proxy in front of it and \
             bind loopback",
        ));
    }
    Ok(())
}

async fn create(name: String, scope: Vec<String>, server: Vec<String>) -> Result<()> {
    let scopes = parse_scopes(&scope)?;
    let client = super::connect().await?;

    // Narrowing is stored by id: a key holding a name would follow a rename.
    let mut servers = Vec::with_capacity(server.len());
    for reference in &server {
        match client.server().status(reference).await {
            Ok(info) => servers.push(info.id),
            Err(_) => bail!("no server matches '{reference}'"),
        }
    }

    let issued = client.remote().create_key(&name, scopes, servers).await?;
    ui::show(View::line(issued.token))?;
    ui::show(View::note(
        "copy this now — only its digest is stored, so it cannot be shown again",
    ))?;
    ui::show(View::detail([
        ("name", issued.key.name.clone()),
        ("prefix", issued.key.prefix.clone()),
        ("scopes", scope_label(&issued.key.scopes)),
        ("servers", server_label(&issued.key.servers)),
    ]))
}

async fn list() -> Result<()> {
    let client = super::connect().await?;
    let keys = client.remote().keys().await?;
    if keys.is_empty() {
        return ui::show(View::note(
            "no keys — mint one with `hestia remote key create --name <what-for> --scope \
             server:read`",
        ));
    }
    let rows = keys
        .iter()
        .map(|key| {
            vec![
                key.prefix.clone(),
                key.name.clone(),
                scope_label(&key.scopes),
                server_label(&key.servers),
                age_label(key.created_unix),
                last_used_label(key),
            ]
        })
        .collect();
    ui::show(View::table(
        "API keys",
        [
            "PREFIX",
            "NAME",
            "SCOPES",
            "SERVERS",
            "CREATED",
            "LAST USED",
        ],
        rows,
    ))
}

async fn revoke(key: String) -> Result<()> {
    let client = super::connect().await?;
    client.remote().revoke_key(&key).await?;
    ui::show(View::line(format!("revoked '{key}'")))
}

fn parse_scopes(raw: &[String]) -> Result<Vec<Scope>> {
    let mut scopes = Vec::with_capacity(raw.len());
    for value in raw {
        match Scope::parse(value) {
            Some(scope) => scopes.push(scope),
            None => bail!(
                "'{value}' is not a scope — pick from {}",
                Scope::ALL
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
    Ok(scopes)
}

fn scope_label(scopes: &[Scope]) -> String {
    scopes
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn server_label(servers: &[String]) -> String {
    match servers.is_empty() {
        true => "every".to_string(),
        false => servers.len().to_string(),
    }
}

fn last_used_label(key: &RemoteKey) -> String {
    match key.last_used_unix {
        0 => "never".to_string(),
        used => age_label(used),
    }
}
