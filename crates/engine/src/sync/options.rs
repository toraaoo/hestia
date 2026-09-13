//! `options.txt`, merged key by key.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;

use super::reconcile;

/// An excluded key is carried through untouched on both sides, so pinning one
/// on a single instance never strips it from the others.
pub fn merge(
    baseline: &Path,
    store: &Path,
    data: &Path,
    excluded: &BTreeSet<String>,
) -> Result<()> {
    let stored = read(store);
    let local = read(data);
    if stored.is_empty() && local.is_empty() {
        return Ok(());
    }
    let base = read(baseline);
    let data_newer = reconcile::newer(data, store);

    let shared: BTreeMap<String, String> = stored
        .keys()
        .chain(local.keys())
        .filter(|key| !excluded.contains(*key))
        .collect::<BTreeSet<&String>>()
        .into_iter()
        .filter_map(|key| {
            let value = reconcile::one(base.get(key), stored.get(key), local.get(key), data_newer)?;
            Some((key.clone(), value.clone()))
        })
        .collect();

    let mut for_data = shared.clone();
    let mut for_store = shared;
    for key in excluded {
        if let Some(value) = local.get(key) {
            for_data.insert(key.clone(), value.clone());
        }
        if let Some(value) = stored.get(key) {
            for_store.insert(key.clone(), value.clone());
        }
    }

    write(data, &for_data)?;
    write(store, &for_store)?;
    reconcile::write_if_changed(baseline, render(&for_store).as_bytes())
}

pub fn read(path: &Path) -> BTreeMap<String, String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    text.lines()
        .filter_map(|line| line.trim().split_once(':'))
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

pub fn write(path: &Path, values: &BTreeMap<String, String>) -> Result<()> {
    reconcile::write_if_changed(path, render(values).as_bytes())
}

fn render(values: &BTreeMap<String, String>) -> String {
    let mut text = String::new();
    for (key, value) in values {
        text.push_str(key);
        text.push(':');
        text.push_str(value);
        text.push('\n');
    }
    text
}
