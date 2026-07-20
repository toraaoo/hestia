//! The plumbing every worker manager shares: event publishing, job ids, and the
//! in-flight key set that admits one job per entry at a time.

use std::borrow::Borrow;
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use ipc::protocol::Event;
use proto::java::{JavaInstallPhase, JavaInstallProgress};
use proto::minecraft::{ProvisionPhase, ProvisionProgress};

/// Below this ratio delta a progress update is dropped, matching Modrinth's
/// `emit_loading` (0.5%): a phase of thousands of tiny units (an instance's
/// assets) otherwise emits an event per unit and floods the socket.
const PROGRESS_EPSILON: f64 = 0.005;

/// A progress payload the coalescer can throttle: a phase discriminant (a
/// change forces a forward so the label switches promptly) and an overall
/// `0.0..=1.0` completion ratio.
pub(super) trait Coalescible {
    type Phase: Copy + PartialEq + Send;
    fn phase(&self) -> Self::Phase;
    fn ratio(&self) -> f64;
}

impl Coalescible for ProvisionProgress {
    type Phase = ProvisionPhase;
    fn phase(&self) -> ProvisionPhase {
        self.phase
    }
    fn ratio(&self) -> f64 {
        ProvisionProgress::ratio(self)
    }
}

impl Coalescible for JavaInstallProgress {
    type Phase = JavaInstallPhase;
    fn phase(&self) -> JavaInstallPhase {
        self.phase
    }
    fn ratio(&self) -> f64 {
        JavaInstallProgress::ratio(self)
    }
}

/// Coalesce a high-frequency progress stream so a phase made of thousands of
/// tiny units (or a per-chunk download) can't flood every subscribed front-end
/// (the desktop re-renders per event; the freeze this fixes). An update is
/// forwarded only when its phase changes, its overall ratio advances past
/// `PROGRESS_EPSILON`, or it is terminal — so the bar still lands on 100% and
/// the label still switches promptly, while the intermediate ticks are dropped.
/// Mirrors the CLI, which throttles at its render layer instead.
pub(super) fn coalesce_progress<P, F>(emit: F) -> impl Fn(&P) + Send + Sync
where
    P: Coalescible,
    F: Fn(&P) + Send + Sync,
{
    let state = Mutex::new(None::<(P::Phase, f64)>);
    move |p: &P| {
        let ratio = p.ratio();
        let mut last = state.lock().unwrap();
        let forward = match *last {
            Some((phase, sent)) => {
                phase != p.phase() || ratio >= 1.0 || (ratio - sent).abs() > PROGRESS_EPSILON
            }
            None => true,
        };
        if forward {
            *last = Some((p.phase(), ratio));
            drop(last);
            emit(p);
        }
    }
}

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

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use proto::java::{JavaInstallPhase, JavaInstallProgress};

    use super::*;

    fn downloading(current: u64, total: u64) -> JavaInstallProgress {
        JavaInstallProgress {
            phase: JavaInstallPhase::Downloading,
            current,
            total,
        }
    }

    #[test]
    fn coalesces_sub_epsilon_java_ticks() {
        let emitted = AtomicUsize::new(0);
        let forward = coalesce_progress(|_: &JavaInstallProgress| {
            emitted.fetch_add(1, Ordering::SeqCst);
        });

        // A per-chunk download flood: one tick per 0.1% of a large archive.
        for i in 0..=1000u64 {
            forward(&downloading(i, 1000));
        }

        // Far fewer than the 1001 ticks: forwarded only past each 0.5% step
        // (~200) plus the first and the terminal 100%.
        let count = emitted.load(Ordering::SeqCst);
        assert!(count > 0, "the first tick and completion must forward");
        assert!(
            count <= 210,
            "sub-epsilon ticks must be dropped, got {count}"
        );
    }

    #[test]
    fn forwards_on_phase_change() {
        let emitted = AtomicUsize::new(0);
        let forward = coalesce_progress(|_: &JavaInstallProgress| {
            emitted.fetch_add(1, Ordering::SeqCst);
        });

        forward(&JavaInstallProgress {
            phase: JavaInstallPhase::Resolving,
            current: 0,
            total: 0,
        });
        // Same zero ratio, new phase: a phase switch always forwards.
        forward(&downloading(0, 0));
        forward(&JavaInstallProgress {
            phase: JavaInstallPhase::Extracting,
            current: 0,
            total: 0,
        });

        assert_eq!(emitted.load(Ordering::SeqCst), 3);
    }
}
