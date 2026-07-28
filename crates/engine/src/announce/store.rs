//! The on-disk half of announcements: the cached feed document and the set of
//! ids the user has dismissed. No network, no filtering — just persistence, so
//! it can be exercised against a temp directory alone.
//!
//! The two files are deliberately separate. `feed.json` is derived and
//! discardable; `seen.json` is user state that must survive a cache wipe, and
//! folding them together would lose dismissals whenever a stale cache was
//! cleared.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// The cached feed exactly as fetched, so it can be re-verified on load rather
/// than trusted because it once passed — the same rule the download cache
/// applies when it re-hashes a blob on the way out.
#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct CachedFeed {
    /// When the fetch that produced `document` succeeded, unix seconds.
    pub fetched: i64,
    /// The raw signed envelope text, byte-for-byte.
    pub document: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Seen {
    dismissed: BTreeSet<String>,
}

pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Store { dir }
    }

    pub fn set_dir(&mut self, dir: PathBuf) {
        self.dir = dir;
    }

    fn feed_path(&self) -> PathBuf {
        self.dir.join("feed.json")
    }

    fn seen_path(&self) -> PathBuf {
        self.dir.join("seen.json")
    }

    /// The cached document, or `None` when absent or unreadable. A corrupt
    /// cache is not an error: the next poll replaces it.
    pub fn load_feed(&self) -> Option<CachedFeed> {
        read_json(&self.feed_path())
    }

    pub fn save_feed(&self, document: &str, fetched: i64) -> Result<()> {
        write_json(
            &self.feed_path(),
            &CachedFeed {
                fetched,
                document: document.to_string(),
            },
        )
    }

    pub fn load_seen(&self) -> BTreeSet<String> {
        read_json::<Seen>(&self.seen_path())
            .unwrap_or_default()
            .dismissed
    }

    pub fn save_seen(&self, dismissed: &BTreeSet<String>) -> Result<()> {
        write_json(
            &self.seen_path(),
            &Seen {
                dismissed: dismissed.clone(),
            },
        )
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(value) => Some(value),
        Err(e) => {
            tracing::debug!(path = %path.display(), error = %e, "discarding unreadable announce state");
            None
        }
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("cannot create the announce directory")?;
    }
    let text = serde_json::to_string_pretty(value).context("cannot encode announce state")?;
    std::fs::write(path, format!("{text}\n")).context("cannot write announce state")
}

#[cfg(test)]
mod tests {
    use super::Store;
    use std::collections::BTreeSet;

    fn store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("announce"));
        (dir, store)
    }

    #[test]
    fn a_missing_store_reads_as_empty() {
        let (_dir, store) = store();
        assert!(store.load_feed().is_none());
        assert!(store.load_seen().is_empty());
    }

    #[test]
    fn the_document_round_trips_byte_for_byte() {
        // The signature covers exact bytes, so the cache must not reformat what
        // it stores or the reload would fail verification.
        let (_dir, store) = store();
        let document = "{\"signature\":\"sig\",\"payload\":\"{\\\"version\\\":1}\"}";
        store.save_feed(document, 42).unwrap();
        let cached = store.load_feed().unwrap();
        assert_eq!(cached.document, document);
        assert_eq!(cached.fetched, 42);
    }

    #[test]
    fn dismissals_survive_a_discarded_feed_cache() {
        let (_dir, store) = store();
        let mut seen = BTreeSet::new();
        seen.insert("an-id".to_string());
        store.save_seen(&seen).unwrap();
        store.save_feed("garbage", 1).unwrap();

        std::fs::remove_file(store.feed_path()).unwrap();
        assert!(store.load_feed().is_none());
        assert!(store.load_seen().contains("an-id"));
    }

    #[test]
    fn a_corrupt_file_is_discarded_rather_than_fatal() {
        let (_dir, store) = store();
        store.save_seen(&BTreeSet::new()).unwrap();
        std::fs::write(store.seen_path(), "not json").unwrap();
        assert!(store.load_seen().is_empty());
    }
}
