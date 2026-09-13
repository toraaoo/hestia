//! `command_history.txt`. It is an append-only log, so the sides are unioned:
//! picking one would drop commands the other typed.

use std::path::Path;

use anyhow::Result;

use super::reconcile;

pub fn merge(baseline: &Path, store: &Path, data: &Path) -> Result<()> {
    let stored = read(store);
    let local = read(data);
    if stored.is_empty() && local.is_empty() {
        return Ok(());
    }

    let mut merged = stored;
    for line in local {
        if !merged.contains(&line) {
            merged.push(line);
        }
    }

    let text = merged.join("\n");
    let bytes = match text.is_empty() {
        true => Vec::new(),
        false => format!("{text}\n").into_bytes(),
    };
    reconcile::write_if_changed(data, &bytes)?;
    reconcile::write_if_changed(store, &bytes)?;
    reconcile::write_if_changed(baseline, &bytes)
}

fn read(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|text| {
            text.lines()
                .map(str::trim_end)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}
