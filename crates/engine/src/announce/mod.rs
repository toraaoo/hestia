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
            .get(endpoint().url)
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
        let source = endpoint();
        let parsed = verify_and_parse(document, source.trust)?;
        let mut inner = self.inner.lock().unwrap();
        let changed = visible(&inner.entries, now) != visible(&parsed.entries, now);
        inner.entries = parsed.entries;
        inner.fetched = now;
        // An unsigned feed is held in memory and never written: the cache is
        // read back under `Trust::Signed`, so caching one would either fail at
        // the next start or — worse — need the exemption widened to the load
        // path, where nothing knows it came from an override.
        if source.trust == Trust::Signed {
            if let Err(e) = inner.store.save_feed(document, now) {
                // The feed is live in memory either way; a cache that cannot be
                // written costs a re-fetch at the next start, not this result.
                tracing::warn!(
                    error = format!("{e:#}"),
                    "cannot cache the announcement feed"
                );
            }
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
        // Always `Signed`: the cache is a file on disk with no provenance, so
        // it is trusted only because it verifies now.
        match verify_and_parse(&cached.document, Trust::Signed) {
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
            // The subsystem knows nothing of the setting that governs it; the
            // flow that owns the gate stamps this on the way out.
            enabled: true,
        }
    }
}

/// The entries this build would show, in a stable order so that reordering the
/// published document is not itself a change. Compared whole rather than by id:
/// a correction to a live announcement's body is exactly the case a poll exists
/// to notice, and an id-only comparison reports it as unchanged.
fn visible(entries: &[feed::Entry], now: i64) -> Vec<&feed::Entry> {
    let ctx = feed::Context::current(now);
    let mut visible: Vec<&feed::Entry> =
        entries.iter().filter(|entry| entry.applies(&ctx)).collect();
    visible.sort_by(|a, b| a.id.cmp(&b.id));
    visible
}

fn verify_and_parse(document: &str, trust: Trust) -> Result<Feed> {
    let envelope: Envelope =
        serde_json::from_str(document).context("malformed announcement envelope")?;
    if trust == Trust::Signed {
        verify_bytes(
            envelope.payload.as_bytes(),
            &envelope.signature,
            common::app::announce_pubkeys(),
        )
        .context("announcement signature verification failed")?;
    } else {
        tracing::warn!(
            "announcement signature not checked — debug build on an overridden endpoint"
        );
    }
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

/// Whether a document has to carry a signature this build trusts.
#[derive(PartialEq, Eq, Clone, Copy)]
enum Trust {
    Signed,
    /// A debug build reading an overridden endpoint. The whole point of the
    /// override is to render a hand-written feed, which by definition nothing
    /// can sign.
    Unchecked,
}

struct Source {
    url: String,
    trust: Trust,
}

/// Where the feed comes from, and what that implies about trusting it.
///
/// `HESTIA_ANNOUNCE_ENDPOINT` lets a **debug** build point at a local file so
/// the surface can be exercised before anything is published — the same escape
/// hatch `HESTIA_SOCK` is for the transport. It also waives the signature
/// check, deliberately: requiring a signature would mean keeping a throwaway
/// keypair just to see a news card render.
///
/// The waiver is narrow by construction. It exists only in a debug binary
/// (`cfg(debug_assertions)` — a release build has no code path to it at all),
/// only for an explicitly overridden endpoint, it is logged at WARN each time,
/// and an unchecked feed is never cached, so it cannot outlive the process that
/// read it. A release build fetching the real endpoint is unaffected.
fn endpoint() -> Source {
    #[cfg(debug_assertions)]
    if let Ok(override_url) = std::env::var("HESTIA_ANNOUNCE_ENDPOINT") {
        if !override_url.trim().is_empty() {
            return Source {
                url: override_url,
                trust: Trust::Unchecked,
            };
        }
    }
    Source {
        url: common::app::ANNOUNCE_ENDPOINT.to_string(),
        trust: Trust::Signed,
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{endpoint, feed, verify_and_parse, visible, Announce, Trust};

    fn entry(id: &str, body: &str) -> feed::Entry {
        feed::Entry {
            id: id.into(),
            body: body.into(),
            ..Default::default()
        }
    }

    #[test]
    fn an_edited_body_counts_as_a_change() {
        // The poll's whole job is to notice a correction to something already
        // published, which keeps its id — comparing ids alone reported that as
        // unchanged and no front-end ever refreshed.
        let before = vec![entry("an-id", "original")];
        let after = vec![entry("an-id", "corrected")];
        assert_ne!(visible(&before, 0), visible(&after, 0));
    }

    #[test]
    fn reordering_the_document_is_not_a_change() {
        let before = vec![entry("a", "one"), entry("b", "two")];
        let after = vec![entry("b", "two"), entry("a", "one")];
        assert_eq!(visible(&before, 0), visible(&after, 0));
    }

    #[test]
    fn an_entry_this_build_never_sees_is_not_a_change() {
        // Targeting is applied before the comparison, so editing an entry aimed
        // at another platform must not wake this one up.
        let other = feed::Entry {
            platforms: vec!["nonesuch".into()],
            ..entry("an-id", "original")
        };
        let edited = feed::Entry {
            body: "corrected".into(),
            ..other.clone()
        };
        assert_eq!(visible(&[other], 0), visible(&[edited], 0));
    }

    #[test]
    fn an_unsigned_feed_is_refused() {
        // No announcement key is compiled in yet, so nothing verifies — the
        // build must show no announcements rather than trust the document.
        let document = r#"{"signature":"","payload":"{\"version\":1,\"entries\":[]}"}"#;
        assert!(verify_and_parse(document, Trust::Signed).is_err());
    }

    #[test]
    fn a_malformed_envelope_is_refused() {
        assert!(verify_and_parse("not json", Trust::Signed).is_err());
    }

    #[test]
    fn the_unchecked_waiver_skips_the_signature_but_not_the_schema() {
        // The debug-override waiver drops the signature check and nothing else:
        // a malformed envelope, bad JSON, or a future schema is still refused.
        let document = r#"{"signature":"","payload":"{\"version\":1,\"entries\":[]}"}"#;
        assert!(verify_and_parse(document, Trust::Unchecked).is_ok());

        let future = r#"{"signature":"","payload":"{\"version\":99,\"entries\":[]}"}"#;
        assert!(verify_and_parse(future, Trust::Unchecked).is_err());
        assert!(verify_and_parse("not json", Trust::Unchecked).is_err());
    }

    #[test]
    fn the_real_endpoint_always_demands_a_signature() {
        // The waiver is reachable only through an overridden endpoint, so the
        // default source must ask for `Signed` even in a debug build.
        assert!(endpoint().trust == Trust::Signed);
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
