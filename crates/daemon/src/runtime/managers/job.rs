//! The plumbing every worker manager shares: event publishing, job ids, and the
//! in-flight key set that admits one job per entry at a time.

use std::borrow::Borrow;
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use ipc::protocol::Event;

pub(super) fn topic_event<E: proto::Topic + serde::Serialize>(event: &E) -> Event {
    Event {
        topic: E::TOPIC.to_string(),
        payload: serde_json::to_value(event).unwrap_or_default(),
    }
}

fn generate_id(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    format!("{prefix}-{}-{}", std::process::id(), n)
}

/// The caller's job id, or a generated one when it left the id empty.
pub(super) fn job_id(id: String, prefix: &str) -> String {
    if id.is_empty() {
        generate_id(prefix)
    } else {
        id
    }
}

/// The keys whose job is still running. A key admits one job at a time.
pub(super) struct InFlight<K> {
    active: Arc<Mutex<HashSet<K>>>,
}

impl<K: Eq + Hash + Clone> InFlight<K> {
    pub(super) fn new() -> Self {
        InFlight {
            active: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub(super) fn contains<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.active.lock().unwrap().contains(key)
    }

    /// Take `key`, or `None` when a job already holds it. The claim releases on
    /// drop, so a job that panics never wedges its key.
    pub(super) fn claim(&self, key: K) -> Option<Claim<K>> {
        if !self.active.lock().unwrap().insert(key.clone()) {
            return None;
        }
        Some(Claim {
            active: self.active.clone(),
            key,
        })
    }
}

pub(super) struct Claim<K: Eq + Hash> {
    active: Arc<Mutex<HashSet<K>>>,
    key: K,
}

impl<K: Eq + Hash> Drop for Claim<K> {
    fn drop(&mut self) {
        self.active.lock().unwrap().remove(&self.key);
    }
}
