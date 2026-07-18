//! Selecting a server and rendering one: the vocabulary the verb modules share.

use anyhow::{bail, Context, Result};
use client::proto::process::{ProcessInfo, ProcessState};
use client::proto::server::{ServerInfo, ServerPingResult};
use client::Client;

use crate::commands::mc;
use crate::ui::{self, View};

pub(super) fn pick_server(
    servers: Vec<ServerInfo>,
    provided: Option<String>,
) -> Result<ServerInfo> {
    if servers.is_empty() {
        bail!("no servers yet (hestia server create)");
    }
    if let Some(reference) = provided {
        return servers
            .into_iter()
            .find(|s| client::proto::naming::reference_matches(&reference, &s.id, &s.name))
            .with_context(|| format!("no server matches '{reference}'"));
    }
    let labels: Vec<String> = servers
        .iter()
        .map(|s| format!("{} ({} {})", s.name, s.flavor, s.game_version))
        .collect();
    let index = ui::select("select a server", &labels)?;
    Ok(servers.into_iter().nth(index).expect("selector index"))
}

pub(super) fn running_process(info: &ServerInfo) -> Option<ProcessInfo> {
    info.process
        .clone()
        .filter(|p| p.state == ProcessState::Running)
}

pub(super) async fn list(client: &Client) -> Result<()> {
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
                address_label(s),
                state_label(s),
            ]
        })
        .collect();
    ui::show(View::table(
        "servers",
        ["NAME", "FLAVOR", "VERSION", "LOADER", "ADDRESS", "STATE"],
        rows,
    ))
}

pub(super) fn show_status(info: &ServerInfo, ping: Option<&ServerPingResult>) -> Result<()> {
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
        ("address", address_label(info)),
        (
            "console",
            if info.console { "yes" } else { "on next start" }.into(),
        ),
        ("state", state_label(info)),
    ];
    if let Some(bytes) = info.disk_bytes {
        rows.push(("disk", ui::human_bytes(bytes)));
    }
    if let Some(ping) = ping {
        rows.push((
            "players",
            format!("{}/{}", ping.players_online, ping.players_max),
        ));
        if !ping.motd.is_empty() {
            rows.push(("motd", ping.motd.clone()));
        }
    }
    ui::show(View::detail(rows))
}

fn address_label(info: &ServerInfo) -> String {
    match info.game_port {
        Some(port) => format!("localhost:{port}"),
        None => "-".into(),
    }
}

fn state_label(info: &ServerInfo) -> String {
    if !info.ready {
        return "provisioning".into();
    }
    mc::process_state_label(&info.process)
}
