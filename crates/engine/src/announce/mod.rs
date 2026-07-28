//! Announcements: the news and notices the launcher fetches from its published
//! feed.
//!
//! Three concerns, one module apiece, so each can be changed without reading
//! the others: [`feed`] is the document format and the targeting rule (pure,
//! no I/O), [`store`] is the cached document and the dismissal set (disk, no
//! network), and this module composes them with the fetch.
//!
//! Adding a targeting dimension is a field on [`feed::Entry`] plus a line in
//! `applies`; adding a severity is an enum variant in `proto`. Neither reaches
//! this file.

mod feed;
mod store;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use proto::announce::{AnnounceListResult, Announcement};

use crate::download::http_client;
use crate::signature::verify_bytes;

use self::feed::{Envelope, Feed};
use self::store::Store;

pub struct Announce {
    inner: Mutex<Inner>,
}

struct Inner {
    store: Store,
    /// Every entry the cached document carries, before targeting. Filtering is
    /// deferred to read time so a build that updates in place stops seeing what
    /// no longer applies to it without re-fetching.
    entries: Vec<feed::Entry>,
    dismissed: BTreeSet<String>,
    fetched: i64,
}

/// What a refresh did, so a poller can decide whether to announce a change and
/// a handler can report staleness without being told "the network failed".
pub struct Refreshed {
    pub result: AnnounceListResult,
    pub changed: bool,
}

impl Announce {
    pub fn new(dir: PathBuf) -> Self {
        let mut inner = Inner {
            store: Store::new(dir),
            entries: Vec::new(),
            dismissed: BTreeSet::new(),
            fetched: 0,
        };
        inner.load();
        Announce {
            inner: Mutex::new(inner),
        }
    }

    pub fn reload(&self, dir: PathBuf) {
        let mut inner = self.inner.lock().unwrap();
        inner.store.set_dir(dir);
        inner.load();
    }

    /// Everything that applies to this build, newest first, dismissed entries
    /// flagged rather than dropped.
    pub fn list(&self) -> AnnounceListResult {
        self.inner.lock().unwrap().list(now_unix())
    }

    /// Remember `ids` as read. Ids that no longer apply are still recorded, so
    /// a re-published entry stays dismissed.
    pub fn dismiss(&self, ids: &[String]) -> Result<AnnounceListResult> {
        let mut inner = self.inner.lock().unwrap();
        for id in ids {
            inner.dismissed.insert(id.clone());
        }
        inner.store.save_seen(&inner.dismissed)?;
        Ok(inner.list(now_unix()))
    }

    /// Fetch, verify, and cache the feed. A fetch failure is **not** an error:
    /// the cached list is returned and `fetched` reports how stale it is — an
    /// unreachable feed is a state, not something to fail a caller over. A
    /// document that arrives but does not verify *is* an error, and the cache
    /// is left alone.
    pub async fn refresh(&self) -> Refreshed {
        match self.fetch().await {
            Ok(document) => match self.ingest(&document, now_unix()) {
                Ok(refreshed) => refreshed,
                Err(e) => {
                    tracing::warn!(error = format!("{e:#}"), "announcement feed rejected");
                    self.unchanged()
                }
            },
            Err(e) => {
                tracing::debug!(error = format!("{e:#}"), "announcement feed unreachable");
                self.unchanged()
            }
        }
    }

    async fn fetch(&self) -> Result<String> {
        Ok(http_client()
            .get(endpoint())
            .send()
            .await
            .context("cannot reach the announcement endpoint")?
            .error_for_status()
            .context("announcement endpoint answered an error")?
            .text()
            .await?)
    }

    /// Verify and adopt a feed document. Separate from [`Announce::fetch`] so
    /// the trust and parsing rules are exercisable without a network.
    fn ingest(&self, document: &str, now: i64) -> Result<Refreshed> {
        let parsed = verify_and_parse(document)?;
        let mut inner = self.inner.lock().unwrap();
        let changed = inner.visible_ids(now) != visible_ids(&parsed.entries, now);
        inner.entries = parsed.entries;
        inner.fetched = now;
        if let Err(e) = inner.store.save_feed(document, now) {
            // The feed is live in memory either way; a cache that cannot be
            // written costs a re-fetch at the next start, not this result.
            tracing::warn!(
                error = format!("{e:#}"),
                "cannot cache the announcement feed"
            );
        }
        Ok(Refreshed {
            result: inner.list(now),
            changed,
        })
    }

    fn unchanged(&self) -> Refreshed {
        Refreshed {
            result: self.list(),
            changed: false,
        }
    }
}

impl Inner {
    /// Read the cache back through the same verification the network path uses:
    /// a cached document is trusted because it verifies now, not because it
    /// verified once.
    fn load(&mut self) {
        self.entries.clear();
        self.fetched = 0;
        self.dismissed = self.store.load_seen();
        let Some(cached) = self.store.load_feed() else {
            return;
        };
        match verify_and_parse(&cached.document) {
            Ok(parsed) => {
                self.entries = parsed.entries;
                self.fetched = cached.fetched;
            }
            Err(e) => {
                tracing::warn!(
                    error = format!("{e:#}"),
                    "cached announcement feed rejected"
                );
            }
        }
    }

    fn list(&self, now: i64) -> AnnounceListResult {
        let ctx = feed::Context::current(now);
        let mut announcements: Vec<Announcement> = self
            .entries
            .iter()
            .filter(|entry| entry.applies(&ctx))
            .map(|entry| {
                entry
                    .clone()
                    .into_announcement(self.dismissed.contains(&entry.id))
            })
            .collect();
        announcements.sort_by(|a, b| b.published.cmp(&a.published).then(a.id.cmp(&b.id)));
        AnnounceListResult {
            announcements,
            fetched: self.fetched,
        }
    }

    fn visible_ids(&self, now: i64) -> Vec<String> {
        visible_ids(&self.entries, now)
    }
}

fn visible_ids(entries: &[feed::Entry], now: i64) -> Vec<String> {
    let ctx = feed::Context::current(now);
    let mut ids: Vec<String> = entries
        .iter()
        .filter(|entry| entry.applies(&ctx))
        .map(|entry| entry.id.clone())
        .collect();
    ids.sort();
    ids
}

fn verify_and_parse(document: &str) -> Result<Feed> {
    let envelope: Envelope =
        serde_json::from_str(document).context("malformed announcement envelope")?;
    verify_bytes(
        envelope.payload.as_bytes(),
        &envelope.signature,
        common::app::announce_pubkeys(),
    )
    .context("announcement signature verification failed")?;
    let parsed: Feed =
        serde_json::from_str(&envelope.payload).context("malformed announcement feed")?;
    if parsed.version > feed::SCHEMA_VERSION {
        bail!(
            "announcement feed schema {} is newer than this build understands ({})",
            parsed.version,
            feed::SCHEMA_VERSION
        );
    }
    Ok(parsed)
}

/// Debug builds may point at a local feed so the surface can be exercised
/// without publishing. The signature is still enforced — the override moves
/// where the document comes from, never whether it is trusted.
fn endpoint() -> String {
    #[cfg(debug_assertions)]
    if let Ok(override_url) = std::env::var("HESTIA_ANNOUNCE_ENDPOINT") {
        if !override_url.trim().is_empty() {
            return override_url;
        }
    }
    common::app::ANNOUNCE_ENDPOINT.to_string()
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{verify_and_parse, Announce};

    #[test]
    fn an_unsigned_feed_is_refused() {
        // No announcement key is compiled in yet, so nothing verifies — the
        // build must show no announcements rather than trust the document.
        let document = r#"{"signature":"","payload":"{\"version\":1,\"entries\":[]}"}"#;
        assert!(verify_and_parse(document).is_err());
    }

    #[test]
    fn a_malformed_envelope_is_refused() {
        assert!(verify_and_parse("not json").is_err());
    }

    #[test]
    fn an_unverifiable_document_leaves_the_cache_alone() {
        let dir = tempfile::tempdir().unwrap();
        let announce = Announce::new(dir.path().join("announce"));
        assert!(announce.ingest("not json", 1).is_err());
        assert!(announce.list().announcements.is_empty());
    }

    #[test]
    fn dismissals_persist_across_a_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("announce");
        let announce = Announce::new(path.clone());
        announce.dismiss(&["an-id".to_string()]).unwrap();

        let reopened = Announce::new(path);
        assert!(reopened.inner.lock().unwrap().dismissed.contains("an-id"));
    }
}
