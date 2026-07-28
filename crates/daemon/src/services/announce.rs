//! Announcements: the news and notices fetched from the published feed.
//!
//! All three channels are plain request/response — a refresh is one small
//! document, not a job. The `announcements.enabled` gate lives in the engine
//! flow, so these handlers never need to check it.

use proto::announce::{AnnounceDismiss, AnnounceDismissParams, AnnounceList, AnnounceRefresh};
use proto::Empty;

use crate::runtime::Channels;

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<AnnounceList, _, _>(|_: Empty, ctx| async move {
        Ok(ctx.runtime.engine().announcements())
    });

    on.handle::<AnnounceDismiss, _, _>(|p: AnnounceDismissParams, ctx| async move {
        let result = ctx
            .runtime
            .engine()
            .dismiss_announcements(&p.ids)
            .map_err(crate::runtime::engine_error)?;
        tracing::debug!(count = p.ids.len(), "announcements dismissed");
        Ok(result)
    });

    on.handle::<AnnounceRefresh, _, _>(|_: Empty, ctx| async move {
        Ok(ctx.runtime.engine().refresh_announcements().await.result)
    });
}
