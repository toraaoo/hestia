//! Selecting an instance and rendering one: the vocabulary the verb modules
//! share.

use anyhow::{bail, Context, Result};
use client::proto::instance::InstanceInfo;
use client::proto::process::{ProcessInfo, ProcessState};
use client::Client;

use crate::commands::mc;
use crate::ui::{self, View};

pub(super) fn pick_instance(
    instances: Vec<InstanceInfo>,
    provided: Option<String>,
) -> Result<InstanceInfo> {
    if instances.is_empty() {
        bail!("no instances yet (hestia instance create)");
    }
    if let Some(reference) = provided {
        return instances
            .into_iter()
            .find(|i| i.id == reference || i.name == reference)
            .with_context(|| format!("no instance matches '{reference}'"));
    }
    let labels: Vec<String> = instances
        .iter()
        .map(|i| format!("{} ({} {})", i.name, i.flavor, i.game_version))
        .collect();
    let index = ui::select("select an instance", &labels)?;
    Ok(instances.into_iter().nth(index).expect("selector index"))
}

pub(super) fn running_process(info: &InstanceInfo) -> Option<ProcessInfo> {
    info.sessions
        .iter()
        .find(|p| p.state == ProcessState::Running)
        .cloned()
}

pub(super) async fn list(client: &Client) -> Result<()> {
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
                mc::sessions_label(&i.sessions),
            ]
        })
        .collect();
    ui::show(View::table(
        "instances",
        ["NAME", "FLAVOR", "VERSION", "LOADER", "STATE"],
        rows,
    ))
}

pub(super) fn show_info(info: &InstanceInfo) -> Result<()> {
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
        ("state", mc::sessions_label(&info.sessions)),
    ]))
}
