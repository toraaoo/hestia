//! Announcements composed with the setting that governs them.
//!
//! These are the *only* way to reach the announce subsystem: the aggregate
//! deliberately exposes no `announce()` getter, because every read has to pass
//! the `announcements.enabled` gate and a getter is an invitation to skip it.
//! Fetching the feed is the daemon's one unprompted outbound request, so a user
//! who turns it off must stop being asked about — not merely stop being shown.

use proto::announce::AnnounceListResult;

use super::Engine;
use crate::announce::Refreshed;

impl Engine {
    fn announcements_enabled(&self) -> bool {
        self.config.settings().announcements.enabled
    }

    /// Everything that applies to this build, or nothing when the feed is off.
    pub fn announcements(&self) -> AnnounceListResult {
        if !self.announcements_enabled() {
            return AnnounceListResult::default();
        }
        self.announce.list()
    }

    /// Mark announcements read. Accepted even with the feed off: the ids are
    /// the user's own state, and dropping them would resurrect everything they
    /// had already dismissed if they turned it back on.
    pub fn dismiss_announcements(&self, ids: &[String]) -> anyhow::Result<AnnounceListResult> {
        let result = self.announce.dismiss(ids)?;
        Ok(if self.announcements_enabled() {
            result
        } else {
            AnnounceListResult::default()
        })
    }

    /// Fetch the feed, unless it is off — in which case no request is made.
    pub async fn refresh_announcements(&self) -> Refreshed {
        if !self.announcements_enabled() {
            return Refreshed {
                result: AnnounceListResult::default(),
                changed: false,
            };
        }
        self.announce.refresh().await
    }
}
