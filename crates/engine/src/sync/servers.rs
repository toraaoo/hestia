//! `servers.dat`, merged entry by entry.
//!
//! Paths here are the directory holding the file: that is what
//! `minecraft::servers` reads and writes, and the name is the same everywhere.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;
use proto::instance::ServerEntry;

use super::reconcile;
use crate::minecraft::servers;

pub fn merge(baseline_dir: &Path, store_dir: &Path, data_dir: &Path) -> Result<()> {
    let stored = servers::read(store_dir);
    let local = servers::read(data_dir);
    if stored.is_empty() && local.is_empty() {
        return Ok(());
    }
    let base = servers::read(baseline_dir);
    let data_newer = reconcile::newer(&servers::path(data_dir), &servers::path(store_dir));

    let merged = settle(&base, &stored, &local, data_newer);

    // Direct-connect rows are the game's scratch, not a list the player curated.
    let for_store: Vec<ServerEntry> = merged
        .iter()
        .filter(|entry| !entry.hidden)
        .cloned()
        .collect();
    let mut for_data = merged;
    for_data.extend(local.iter().filter(|entry| entry.hidden).cloned());

    if for_data != local {
        servers::write(data_dir, &for_data)?;
    }
    if for_store != stored {
        servers::write(store_dir, &for_store)?;
    }
    if for_store != base {
        servers::write(baseline_dir, &for_store)?;
    }
    Ok(())
}

/// The player's own name for a row is its identity, so editing an address is an
/// edit to that entry rather than a new one.
fn key(entry: &ServerEntry) -> String {
    entry.name.trim().to_lowercase()
}

fn find<'a>(list: &'a [ServerEntry], wanted: &str) -> Option<&'a ServerEntry> {
    list.iter().find(|entry| key(entry) == wanted)
}

fn settle(
    base: &[ServerEntry],
    stored: &[ServerEntry],
    local: &[ServerEntry],
    data_newer: bool,
) -> Vec<ServerEntry> {
    let mut seen = BTreeSet::new();
    stored
        .iter()
        .chain(local.iter())
        .filter(|entry| !entry.hidden)
        .map(key)
        .filter(|name| seen.insert(name.clone()))
        .filter_map(|name| {
            reconcile::one(
                find(base, &name),
                find(stored, &name),
                find(local, &name),
                data_newer,
            )
            .cloned()
        })
        .collect()
}
