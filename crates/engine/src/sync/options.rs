//! `options.txt`, merged key by key over the file the game wrote.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;

use super::document::Document;
use super::reconcile;

/// An excluded key is never read or written, so pinning one on a single
/// instance cannot strip it from the others.
pub fn merge(
    baseline: &Path,
    store: &Path,
    data: &Path,
    excluded: &BTreeSet<String>,
) -> Result<()> {
    let stored = Document::read(store)?;
    let local = Document::read(data)?;
    if stored.is_none() && local.is_none() {
        return Ok(());
    }
    let mut stored = stored.unwrap_or_default();
    let mut local = local.unwrap_or_default();
    let base = Document::read(baseline).ok().flatten().unwrap_or_default();
    let data_newer = reconcile::newer(data, store);

    let shared: BTreeSet<String> = stored
        .keys()
        .chain(local.keys())
        .filter(|key| !excluded.contains(*key))
        .map(str::to_string)
        .collect();

    for key in shared {
        let settled = reconcile::one(
            base.get(&key).map(str::to_string).as_ref(),
            stored.get(&key).map(str::to_string).as_ref(),
            local.get(&key).map(str::to_string).as_ref(),
            data_newer,
        )
        .cloned();
        let Some(value) = settled else {
            continue;
        };
        stored.set(&key, &value);
        local.set(&key, &value);
    }

    reconcile::write_if_changed(data, local.render().as_bytes())?;
    reconcile::write_if_changed(store, stored.render().as_bytes())?;
    reconcile::write_if_changed(baseline, stored.render().as_bytes())
}

pub fn read(path: &Path) -> BTreeMap<String, String> {
    let Ok(Some(document)) = Document::read(path) else {
        return BTreeMap::new();
    };
    document
        .keys()
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(|key| {
            let value = document.get(&key)?.to_string();
            Some((key, value))
        })
        .collect()
}

pub fn set(path: &Path, key: &str, value: &str) -> Result<()> {
    let mut document = Document::read(path)?.unwrap_or_default();
    document.set(key, value);
    reconcile::write_if_changed(path, document.render().as_bytes())
}
