//! Shared helpers for the `server` and `instance` command trees: resolving a
//! flavor (interactively when not given) and rendering flavor/version lists.

use anyhow::{bail, Result};
use client::proto::minecraft::{Flavor, GameVersion, VersionKind};

use crate::ui::{self, View};

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
