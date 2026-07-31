//! The announcement poll: fetch the feed at startup, then every few hours.
//!
//! Deliberately its own loop rather than a branch inside the backup scheduler —
//! the two share nothing but a timer, and the backup loop's tick is a minute
//! because a backup schedule is expressed in minutes.
//!
//! A poll that changes what applies to this build publishes `announce.changed`,
//! so a front-end refreshes its badge without holding a query open. A poll that
//! fails publishes nothing: the cached list is still what the daemon serves, and
//! an unreachable feed is a state the `fetched` timestamp already reports.

use std::sync::Arc;
use std::time::Duration;

use proto::announce::AnnounceChangedEvent;

use super::managers::topic_event;
use super::Runtime;

/// Announcements are edited by hand and read at a glance; polling more often
/// than this spends requests on a document that changes a few times a year.
#[cfg(not(debug_assertions))]
const INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// A debug run serves `news/` off the local disk, where the document changes
/// every time an entry is edited — six hours there means never. The short tick
/// exists only in a debug binary, so it cannot reach a shipped build.
#[cfg(debug_assertions)]
const INTERVAL: Duration = Duration::from_secs(30);

pub fn spawn_announcement_poller(runtime: Arc<Runtime>) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(INTERVAL);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            // The first tick completes immediately, so a daemon start is also a
            // poll — the point at which a stale cache is most worth replacing.
            tick.tick().await;
            poll(&runtime).await;
        }
    });
}

async fn poll(runtime: &Runtime) {
    let refreshed = runtime.engine().refresh_announcements().await;
    if !refreshed.changed {
        return;
    }
    let unread = refreshed
        .result
        .announcements
        .iter()
        .filter(|a| !a.dismissed)
        .count() as u32;
    tracing::info!(
        total = refreshed.result.announcements.len(),
        unread,
        "announcements changed"
    );
    runtime
        .hub()
        .publish(&topic_event(&AnnounceChangedEvent { unread }));
}
