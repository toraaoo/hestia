//! Copied sync targets, reconciled three-way against a **baseline** — the
//! content the instance and the store last agreed on. Each side is asked
//! whether it moved since that agreement; an mtime is stamped by the copy
//! rather than by the edit behind it, so it survives only as the tiebreak when
//! both sides moved ([0069](../../../docs/decisions/0069-sync-reconciles-against-a-baseline.md)).
//!
//! A missing side is never an edit: it is filled from the other, never
//! propagated as a deletion. Same for a key only one side knows — instances on
//! different game versions have different option sets.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use anyhow::{Context, Result};

/// `options.txt` keys kept entry-local — pack selection must not leak between
/// instances through the shared store (mirrors Pandora's `options.txt` handling).
const LOCAL_OPTION_KEYS: &[&str] = &["resourcePacks", "incompatibleResourcePacks"];

/// Which way a target settles: the store onto the instance, or the reverse.
enum Settle {
    Pull,
    Push,
}

/// Reconcile one whole-file target, then record what the two sides settled on.
pub fn reconcile(baseline: &Path, store: &Path, data: &Path) -> Result<()> {
    let stored = read(store);
    let local = read(data);
    if stored.is_none() && local.is_none() {
        return Ok(());
    }
    if stored == local {
        return record(baseline, local.as_deref().unwrap_or_default());
    }

    let settle = if local.is_none() {
        Settle::Pull
    } else if stored.is_none() {
        Settle::Push
    } else {
        let base = read(baseline);
        match (base != stored, base != local) {
            (true, false) => Settle::Pull,
            (false, true) => Settle::Push,
            _ if newer(data, store) => Settle::Push,
            _ => Settle::Pull,
        }
    };
    let agreed = match settle {
        Settle::Pull => {
            copy_file(store, data)?;
            stored
        }
        Settle::Push => {
            copy_file(data, store)?;
            local
        }
    };
    record(baseline, agreed.as_deref().unwrap_or_default())
}

/// Reconcile `options.txt` key by key: the side that changed a key since the
/// baseline wins it, the newer file wins a key both changed, and a key only one
/// side knows is carried through.
pub fn merge_options(baseline: &Path, store: &Path, data: &Path) -> Result<()> {
    let stored = read_options(store);
    let local = read_options(data);
    if stored.is_empty() && local.is_empty() {
        return Ok(());
    }
    let base = read_options(baseline);
    let data_newer = newer(data, store);

    let keys: BTreeSet<&String> = stored.keys().chain(local.keys()).collect();
    let merged: BTreeMap<String, String> = keys
        .into_iter()
        .filter_map(|key| {
            let value = resolve(base.get(key), stored.get(key), local.get(key), data_newer)?;
            Some((key.clone(), value.clone()))
        })
        .collect();

    let mut for_data = merged.clone();
    for key in LOCAL_OPTION_KEYS {
        match local.get(*key) {
            Some(value) => for_data.insert(key.to_string(), value.clone()),
            None => for_data.remove(*key),
        };
    }
    let mut for_store = merged;
    for key in LOCAL_OPTION_KEYS {
        for_store.remove(*key);
    }

    write_options(data, &for_data)?;
    write_options(store, &for_store)?;
    record(baseline, render(&for_store).as_bytes())
}

/// Record the instance's *current* content as the agreement, so the next
/// reconcile reads every disagreement as the store's change and settles it the
/// store's way — what an instance rejoining sharing gets.
pub fn defer_to_store(baseline: &Path, data: &Path) -> Result<()> {
    record(baseline, &read(data).unwrap_or_default())
}

/// Which value for one key survives the pass.
fn resolve<'a>(
    base: Option<&String>,
    stored: Option<&'a String>,
    local: Option<&'a String>,
    data_newer: bool,
) -> Option<&'a String> {
    match (stored, local) {
        (Some(s), Some(d)) if s == d => Some(s),
        (Some(s), Some(d)) => match base {
            Some(b) if b == s => Some(d),
            Some(b) if b == d => Some(s),
            _ => Some(if data_newer { d } else { s }),
        },
        (Some(s), None) => Some(s),
        (None, other) => other,
    }
}

/// Copy `from` onto `to`, carrying the source's modification time across: the
/// stamp is the tiebreak between two edited sides, so it must describe the edit
/// rather than the copy that moved it.
pub fn copy_file(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    fs::copy(from, to)
        .with_context(|| format!("cannot copy {} to {}", from.display(), to.display()))?;
    if let Some(time) = mtime(from) {
        fs::File::options()
            .write(true)
            .open(to)
            .and_then(|file| file.set_modified(time))
            .with_context(|| format!("cannot stamp {}", to.display()))?;
    }
    Ok(())
}

fn record(baseline: &Path, agreed: &[u8]) -> Result<()> {
    if read(baseline).is_some_and(|current| current == agreed) {
        return Ok(());
    }
    if let Some(parent) = baseline.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    fs::write(baseline, agreed).with_context(|| format!("cannot write {}", baseline.display()))
}

fn read(path: &Path) -> Option<Vec<u8>> {
    fs::read(path).ok()
}

fn newer(a: &Path, b: &Path) -> bool {
    match (mtime(a), mtime(b)) {
        (Some(ta), Some(tb)) => ta >= tb,
        (Some(_), None) => true,
        _ => false,
    }
}

fn mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn read_options(path: &Path) -> BTreeMap<String, String> {
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    text.lines()
        .filter_map(|line| line.trim().split_once(':'))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
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

/// A rewrite that changes nothing would still stamp the file, and the store's
/// stamp is what every other instance's tiebreak reads.
fn write_options(path: &Path, values: &BTreeMap<String, String>) -> Result<()> {
    let text = render(values);
    if fs::read_to_string(path).is_ok_and(|current| current == text) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    fs::write(path, text).with_context(|| format!("cannot write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str) -> Option<String> {
        Some(text.to_string())
    }

    /// The store's file is the newer of the two, but it never moved off the
    /// baseline — only the instance changed the key.
    #[test]
    fn one_side_changing_a_key_wins_over_a_newer_file() {
        let base = value("2");
        let stored = value("2");
        let local = value("4");
        assert_eq!(
            resolve(base.as_ref(), stored.as_ref(), local.as_ref(), false),
            local.as_ref()
        );
    }

    #[test]
    fn both_changing_one_key_falls_back_to_the_clock() {
        let base = value("2");
        let stored = value("3");
        let local = value("4");
        assert_eq!(
            resolve(base.as_ref(), stored.as_ref(), local.as_ref(), true),
            local.as_ref()
        );
        assert_eq!(
            resolve(base.as_ref(), stored.as_ref(), local.as_ref(), false),
            stored.as_ref()
        );
    }

    #[test]
    fn a_key_only_one_side_knows_is_kept() {
        let only_store = value("on");
        assert_eq!(
            resolve(None, only_store.as_ref(), None, true),
            only_store.as_ref()
        );
        let only_data = value("off");
        assert_eq!(
            resolve(None, None, only_data.as_ref(), false),
            only_data.as_ref()
        );
        assert_eq!(resolve(None, None, None, true), None);
    }
}
