use std::time::Duration;

use ipc::errors::IpcError;
use proto::announce::AnnounceListResult;

use crate::session::Session;

/// A refresh fetches the feed over the network before answering, so it needs
/// more than the default call timeout — the same allowance `modpack.resolve`
/// takes for its inline download.
const REFRESH_TIMEOUT: Duration = Duration::from_secs(30);

pub struct Announce<'a> {
    pub(crate) session: &'a Session,
}

impl Announce<'_> {
    /// Everything that applies to this build, newest first. Answers from the
    /// daemon's cache — it never waits on the network.
    pub async fn list(&self) -> Result<AnnounceListResult, IpcError> {
        self.session
            .call::<proto::announce::AnnounceList>(&proto::Empty {})
            .await
    }

    /// Mark announcements read.
    pub async fn dismiss(&self, ids: Vec<String>) -> Result<AnnounceListResult, IpcError> {
        self.session
            .call::<proto::announce::AnnounceDismiss>(&proto::announce::AnnounceDismissParams {
                ids,
            })
            .await
    }

    /// Fetch the feed now rather than waiting for the daemon's poll. Answers
    /// from cache if the fetch fails, so this does not error on a dead network.
    pub async fn refresh(&self) -> Result<AnnounceListResult, IpcError> {
        self.session
            .call_with_timeout::<proto::announce::AnnounceRefresh>(
                &proto::Empty {},
                REFRESH_TIMEOUT,
            )
            .await
    }
}
