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

impl Coalescible for proto::download::DownloadProgress {
    type Phase = ();
    fn phase(&self) {}
    fn ratio(&self) -> f64 {
        match self.total {
            0 => 0.0,
            total => self.downloaded as f64 / total as f64,
        }
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

pub(crate) fn topic_event<E: proto::Topic + serde::Serialize>(event: &E) -> Event {
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
    use std::time::Duration;

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

    /// A job family invented for the tests: the runner never looks inside a
    /// payload, so one stand-in family exercises every path.
    mod family {
        use serde::Serialize;

        #[derive(Serialize, Clone)]
        pub struct Progress {
            pub done: u64,
        }
        impl super::super::Coalescible for Progress {
            type Phase = ();
            fn phase(&self) {}
            fn ratio(&self) -> f64 {
                self.done as f64 / 1000.0
            }
        }

        #[derive(Serialize)]
        pub struct ProgressEvent {
            pub id: String,
            pub done: u64,
        }
        impl proto::Topic for ProgressEvent {
            const TOPIC: &'static str = "test.progress";
        }

        #[derive(Serialize)]
        pub struct DoneEvent {
            pub id: String,
        }
        impl proto::Topic for DoneEvent {
            const TOPIC: &'static str = "test.done";
        }

        #[derive(Serialize)]
        pub struct CancelledEvent {
            pub id: String,
        }
        impl proto::Topic for CancelledEvent {
            const TOPIC: &'static str = "test.cancelled";
        }

        #[derive(Serialize)]
        pub struct ErrorEvent {
            pub id: String,
            pub message: String,
        }
        impl proto::Topic for ErrorEvent {
            const TOPIC: &'static str = "test.error";
        }
    }

    struct Harness {
        runner: Runner<String>,
        events: tokio::sync::mpsc::UnboundedReceiver<String>,
        _home: tempfile::TempDir,
    }

    fn harness() -> Harness {
        let home = tempfile::tempdir().expect("temp home");
        let engine = Arc::new(engine::Engine::new(Some(home.path())));
        let hub = Arc::new(crate::runtime::EventHub::default());
        let (tx, events) = tokio::sync::mpsc::unbounded_channel();
        hub.subscribe(1, tx, None);
        Harness {
            runner: Runner::new(engine, hub, Cancellations::new()),
            events,
            _home: home,
        }
    }

    /// The spec for the stand-in family, with `key` and `cancelled` varied per
    /// test — everything else the runner owns.
    fn spec(key: Option<String>, cancelled: bool) -> Spec<String, family::Progress, ()> {
        Spec {
            id: String::new(),
            prefix: "test",
            key,
            progress: progress_event(|id, p: &family::Progress| family::ProgressEvent {
                id,
                done: p.done,
            }),
            done: settle(|id, ()| family::DoneEvent { id }),
            cancelled: cancelled.then(|| settle(|id, ()| family::CancelledEvent { id })),
            error: settle(|id, e: anyhow::Error| family::ErrorEvent {
                id,
                message: format!("{e:#}"),
            }),
        }
    }

    /// The topics published so far, drained.
    async fn topics(events: &mut tokio::sync::mpsc::UnboundedReceiver<String>) -> Vec<String> {
        let mut seen = Vec::new();
        while let Ok(Some(frame)) =
            tokio::time::timeout(Duration::from_millis(500), events.recv()).await
        {
            let value: serde_json::Value = serde_json::from_str(&frame).expect("a frame");
            seen.push(value["event"].as_str().unwrap_or_default().to_string());
        }
        seen
    }

    #[tokio::test]
    async fn a_key_admits_one_job_at_a_time() {
        let harness = harness();
        let (gate, held) = tokio::sync::oneshot::channel::<()>();

        let first = harness
            .runner
            .start(spec(Some("entry".into()), true), move |_, _| {
                Box::pin(async move {
                    let _ = held.await;
                    Ok(())
                })
            });
        assert!(first.is_some(), "the first job takes the key");

        let second = harness
            .runner
            .start(spec(Some("entry".into()), true), |_, _| {
                Box::pin(async { Ok(()) })
            });
        assert!(second.is_none(), "the second is refused while it is held");
        assert!(harness.runner.in_flight("entry"));

        let _ = gate.send(());
    }

    #[tokio::test]
    async fn a_finished_job_releases_its_key() {
        let mut harness = harness();
        harness
            .runner
            .start(spec(Some("entry".into()), true), |_, _| {
                Box::pin(async { Ok(()) })
            })
            .expect("started");
        topics(&mut harness.events).await;

        assert!(!harness.runner.in_flight("entry"));
        assert!(harness
            .runner
            .start(spec(Some("entry".into()), true), |_, _| Box::pin(async {
                Ok(())
            }))
            .is_some());
    }

    #[tokio::test]
    async fn a_keyless_family_never_refuses() {
        let harness = harness();
        for _ in 0..3 {
            assert!(harness
                .runner
                .start(spec(None, true), |_, _| Box::pin(async {
                    tokio::task::yield_now().await;
                    Ok(())
                }))
                .is_some());
        }
    }

    #[tokio::test]
    async fn progress_is_coalesced_and_the_job_settles_on_done() {
        let mut harness = harness();
        harness
            .runner
            .start(spec(None, true), |_, reporter| {
                Box::pin(async move {
                    let report = reporter.checked();
                    // A flood of sub-epsilon ticks, as an assets pass emits.
                    for done in 0..=1000 {
                        report(&family::Progress { done })?;
                    }
                    Ok(())
                })
            })
            .expect("started");

        let seen = topics(&mut harness.events).await;
        assert_eq!(seen.last().map(String::as_str), Some("test.done"));
        let progress = seen.iter().filter(|t| *t == "test.progress").count();
        assert!(
            (1..=210).contains(&progress),
            "every family is coalesced by the runner, got {progress} of 1001 ticks"
        );
    }

    #[tokio::test]
    async fn a_cancelled_job_settles_apart_from_a_failure() {
        let mut harness = harness();
        harness
            .runner
            .start(spec(None, true), |_, reporter| {
                Box::pin(async move {
                    reporter.cancel().cancel();
                    reporter.cancel().check()?;
                    Ok(())
                })
            })
            .expect("started");

        assert_eq!(topics(&mut harness.events).await, vec!["test.cancelled"]);
    }

    #[tokio::test]
    async fn a_family_with_no_cancelled_topic_reports_the_failure() {
        let mut harness = harness();
        harness
            .runner
            .start(spec(None, false), |_, reporter| {
                Box::pin(async move {
                    reporter.cancel().cancel();
                    reporter.cancel().check()?;
                    Ok(())
                })
            })
            .expect("started");

        assert_eq!(topics(&mut harness.events).await, vec!["test.error"]);
    }

    #[tokio::test]
    async fn a_failed_job_settles_on_its_error_topic() {
        let mut harness = harness();
        harness
            .runner
            .start(spec(None, true), |_, _| {
                Box::pin(async { Err(anyhow::anyhow!("no")) })
            })
            .expect("started");

        assert_eq!(topics(&mut harness.events).await, vec!["test.error"]);
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

/// Every cancellable job currently running, keyed by its job id — the same id
/// its progress and terminal events carry, so `job.cancel` needs nothing else to
/// find it.
///
/// The registration guard releases on drop, exactly as [`Claim`] does, so a job
/// that panics leaves no token behind for a later job to inherit.
#[derive(Clone, Default)]
pub struct Cancellations {
    running: Arc<Mutex<std::collections::HashMap<String, engine::Cancel>>>,
}

impl Cancellations {
    pub fn new() -> Self {
        Cancellations::default()
    }

    /// Register `id` and hand back its token plus the guard that unregisters it.
    pub(super) fn register(&self, id: &str) -> (engine::Cancel, Registered) {
        let cancel = engine::Cancel::new();
        self.running
            .lock()
            .unwrap()
            .insert(id.to_string(), cancel.clone());
        (
            cancel,
            Registered {
                running: self.running.clone(),
                id: id.to_string(),
            },
        )
    }

    /// Raise `id`'s cancel flag. False when no such job is running — it already
    /// finished, or never existed; a normal race, not an error.
    pub fn cancel(&self, id: &str) -> bool {
        match self.running.lock().unwrap().get(id) {
            Some(cancel) => {
                cancel.cancel();
                tracing::info!(job = %id, "job cancellation requested");
                true
            }
            None => {
                tracing::debug!(job = %id, "nothing to cancel");
                false
            }
        }
    }
}

pub(super) struct Registered {
    running: Arc<Mutex<std::collections::HashMap<String, engine::Cancel>>>,
    id: String,
}

impl Drop for Registered {
    fn drop(&mut self) {
        self.running.lock().unwrap().remove(&self.id);
    }
}

/// What a job reports through and is cancelled by. The reporter is already
/// coalesced, so a family never decides how often to publish.
pub(super) struct Reporter<'a, P> {
    publish: &'a (dyn Fn(&P) + Send + Sync),
    cancel: &'a engine::Cancel,
}

impl<'a, P> Reporter<'a, P> {
    pub(super) fn cancel(&self) -> &'a engine::Cancel {
        self.cancel
    }

    /// A progress callback that checkpoints, for an engine call whose reporter
    /// is fallible (a download stops between chunks).
    pub(super) fn checked(&self) -> impl Fn(&P) -> anyhow::Result<()> + Send + Sync + '_ {
        move |progress: &P| {
            self.cancel.check()?;
            (self.publish)(progress);
            Ok(())
        }
    }
}

impl<'a> Reporter<'a, ProvisionProgress> {
    pub(super) fn job(&self) -> engine::Job<'a> {
        engine::Job::new(self.publish, self.cancel)
    }
}

type Work<'a, T> =
    std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<T>> + Send + 'a>>;
type Progress<P> = Box<dyn Fn(String, &P) -> Event + Send + Sync>;
type Settle<T> = Box<dyn FnOnce(String, T) -> Event + Send>;

/// Everything one job family differs by. Claiming, spawning, coalescing,
/// cancellation, terminal classification and logging are the runner's.
pub(super) struct Spec<K, P, T> {
    /// The caller's job id; empty generates one from `prefix`.
    pub id: String,
    pub prefix: &'static str,
    /// The in-flight key. `None` admits any number of this family at once.
    pub key: Option<K>,
    pub progress: Progress<P>,
    pub done: Settle<T>,
    /// `None` reports a cancellation through `error`, for a family with no
    /// cancelled topic of its own.
    pub cancelled: Option<Settle<()>>,
    pub error: Settle<anyhow::Error>,
}

/// The plumbing every worker manager runs its jobs through.
pub(super) struct Runner<K> {
    engine: Arc<engine::Engine>,
    hub: Arc<crate::runtime::EventHub>,
    cancellations: Cancellations,
    active: InFlight<K>,
}

impl<K: Eq + Hash + Clone + Send + std::fmt::Debug + 'static> Runner<K> {
    pub(super) fn new(
        engine: Arc<engine::Engine>,
        hub: Arc<crate::runtime::EventHub>,
        cancellations: Cancellations,
    ) -> Self {
        Runner {
            engine,
            hub,
            cancellations,
            active: InFlight::new(),
        }
    }

    pub(super) fn in_flight<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.active.contains(key)
    }

    /// Run `work` off-thread, publishing its progress and terminal outcome.
    /// Returns the job id, or `None` when the key is already claimed.
    pub(super) fn start<P, T, F>(&self, spec: Spec<K, P, T>, work: F) -> Option<String>
    where
        P: Coalescible + Send + Sync + 'static,
        T: Send + 'static,
        F: for<'a> FnOnce(&'a engine::Engine, Reporter<'a, P>) -> Work<'a, T> + Send + 'static,
    {
        let id = job_id(spec.id, spec.prefix);
        let claim = match spec.key {
            Some(key) => match self.active.claim(key.clone()) {
                Some(claim) => Some(claim),
                None => {
                    tracing::debug!(key = ?key, kind = spec.prefix, "job already in flight");
                    return None;
                }
            },
            None => None,
        };

        let engine = self.engine.clone();
        let hub = self.hub.clone();
        let cancellations = self.cancellations.clone();
        let (job_id, kind) = (id.clone(), spec.prefix);
        let (progress, done, cancelled, error) =
            (spec.progress, spec.done, spec.cancelled, spec.error);
        tracing::info!(job = %id, kind, "job started");

        tokio::spawn(async move {
            let _claim = claim;
            let (cancel, _registered) = cancellations.register(&job_id);

            let publish_hub = hub.clone();
            let publish_id = job_id.clone();
            let publish: Box<dyn Fn(&P) + Send + Sync> =
                Box::new(coalesce_progress(move |p: &P| {
                    publish_hub.publish(&progress(publish_id.clone(), p));
                }));

            let outcome = work(
                &engine,
                Reporter {
                    publish: publish.as_ref(),
                    cancel: &cancel,
                },
            )
            .await;

            let event = match outcome {
                Ok(value) => {
                    tracing::info!(job = %job_id, kind, "job done");
                    done(job_id.clone(), value)
                }
                Err(e) if engine::is_cancelled(&e) && cancelled.is_some() => {
                    tracing::info!(job = %job_id, kind, "job cancelled");
                    cancelled.expect("checked")(job_id.clone(), ())
                }
                Err(e) => {
                    tracing::error!(job = %job_id, kind, error = format!("{e:#}"), "job failed");
                    error(job_id.clone(), e)
                }
            };
            hub.publish(&event);
        });
        Some(id)
    }
}

/// A progress-event builder for a family whose event is `{ id, progress }`.
pub(super) fn progress_event<P, E, F>(build: F) -> Progress<P>
where
    E: proto::Topic + serde::Serialize,
    F: Fn(String, &P) -> E + Send + Sync + 'static,
{
    Box::new(move |id, progress| topic_event(&build(id, progress)))
}

/// A terminal-event builder.
pub(super) fn settle<T, E, F>(build: F) -> Settle<T>
where
    E: proto::Topic + serde::Serialize,
    F: FnOnce(String, T) -> E + Send + 'static,
{
    Box::new(move |id, value| topic_event(&build(id, value)))
}
