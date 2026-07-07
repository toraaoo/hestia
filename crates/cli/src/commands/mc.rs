//! Shared helpers for the `server` and `instance` command trees: resolving a
//! flavor (interactively when not given) and rendering flavor/version lists.

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use client::proto::minecraft::{ConfigEntry, Flavor, GameVersion, VersionKind};
use client::proto::process::{ProcessInfo, ProcessState};

use crate::ui::{self, View};

/// The shared `config` grammar for a server/instance: `get`/`set`/`list`,
/// mirroring `hestia config`.
#[derive(Subcommand)]
pub enum ConfigCmd {
    /// Print the value of a setting
    Get {
        /// Setting key (e.g. memory, jvm-args, or a server.properties key)
        key: String,
    },
    /// Set a setting (an empty value clears it)
    Set {
        /// Setting key
        key: String,
        /// New value; a JVM-flag string may start with '-'
        #[arg(allow_hyphen_values = true)]
        value: String,
    },
    /// List every setting
    #[command(visible_alias = "ls")]
    List,
}

/// Render `config list` entries as a KEY/VALUE table.
pub fn show_config_entries(title: impl Into<String>, entries: Vec<ConfigEntry>) -> Result<()> {
    let rows = entries
        .into_iter()
        .map(|e| vec![e.key, e.value])
        .collect::<Vec<_>>();
    ui::show(View::table(title, ["KEY", "VALUE"], rows))
}

/// One-word state for a managed server/instance from its supervised process
/// snapshot (absent means it has never been started).
pub fn process_state_label(process: &Option<ProcessInfo>) -> String {
    match process {
        Some(p) if p.state == ProcessState::Running => format!("running (pid {})", p.pid),
        Some(_) | None => "stopped".to_string(),
    }
}

/// Return the chosen flavor id: validated when `provided`, otherwise picked from
/// an interactive selector.
pub fn pick_flavor(flavors: Vec<Flavor>, provided: Option<String>) -> Result<String> {
    if flavors.is_empty() {
        bail!("no flavors are available");
    }
    if let Some(id) = provided {
        if flavors.iter().any(|f| f.id == id) {
            return Ok(id);
        }
        let ids: Vec<&str> = flavors.iter().map(|f| f.id.as_str()).collect();
        bail!("unknown flavor '{id}' (available: {})", ids.join(", "));
    }
    let labels: Vec<String> = flavors.iter().map(|f| f.name.clone()).collect();
    let index = ui::select("select a flavor", &labels)?;
    Ok(flavors[index].id.clone())
}

/// Return the chosen version id: validated against the flavor's catalogue when
/// `provided`, otherwise picked from an interactive selector over the releases.
pub fn pick_version(versions: Vec<GameVersion>, provided: Option<String>) -> Result<String> {
    if versions.is_empty() {
        bail!("no versions are available");
    }
    if let Some(id) = provided {
        if versions.iter().any(|v| v.id == id) {
            return Ok(id);
        }
        bail!("unknown version '{id}' (see `hestia server|instance versions`)");
    }
    let releases: Vec<&GameVersion> = versions
        .iter()
        .filter(|v| v.kind == VersionKind::Release)
        .collect();
    let pool = if releases.is_empty() {
        versions.iter().collect()
    } else {
        releases
    };
    let labels: Vec<String> = pool.iter().map(|v| v.id.clone()).collect();
    let index = ui::select("select a version", &labels)?;
    Ok(pool[index].id.clone())
}

/// Interactive fallback for a missing `--downgrade`; errors when stdin is not
/// a terminal so scripts must pass the flag explicitly.
pub fn confirm_downgrade(name: &str, data: &str, from: &str, to: &str) -> Result<()> {
    let choice = ui::select(
        &format!(
            "{to} is older than {from}, and Minecraft cannot load {data} \
             written by a newer version"
        ),
        &[format!("downgrade '{name}'"), "cancel".to_string()],
    )
    .context("pass --downgrade to allow a downgrade")?;
    if choice != 0 {
        bail!("downgrade cancelled");
    }
    Ok(())
}

/// The non-interactive form of the selector: the available flavors as a table.
pub fn show_flavors(flavors: &[Flavor]) -> Result<()> {
    if flavors.is_empty() {
        return ui::show(View::note("no flavors are available"));
    }
    let rows: Vec<Vec<String>> = flavors
        .iter()
        .map(|f| vec![f.id.clone(), f.name.clone()])
        .collect();
    ui::show(View::table("flavors", ["ID", "NAME"], rows))
}

/// Show a version table, releases only unless `all` includes snapshots and old
/// versions. Long lists page on a terminal and dump plainly when piped.
pub fn show_versions(flavor: &str, versions: Vec<GameVersion>, all: bool) -> Result<()> {
    let rows: Vec<Vec<String>> = versions
        .iter()
        .filter(|v| all || v.kind == VersionKind::Release)
        .map(|v| vec![v.id.clone(), kind_label(v.kind).to_string()])
        .collect();
    if rows.is_empty() {
        return ui::show(View::note("no versions available"));
    }
    ui::show(View::table(
        format!("{flavor} versions"),
        ["VERSION", "TYPE"],
        rows,
    ))
}

fn kind_label(kind: VersionKind) -> &'static str {
    match kind {
        VersionKind::Release => "release",
        VersionKind::Snapshot => "snapshot",
        VersionKind::OldBeta => "beta",
        VersionKind::OldAlpha => "alpha",
    }
}
