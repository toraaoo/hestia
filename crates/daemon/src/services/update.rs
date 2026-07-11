//! Self-update: the released-version check and the signed installer download.

use proto::update::{UpdateCheck, UpdateDownload, UpdateDownloadResult};
use proto::Empty;

use crate::runtime::{Channels, ServiceError};

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<UpdateCheck, _, _>(|_: Empty, ctx| async move {
        ctx.runtime
            .engine()
            .update()
            .check()
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))
    });

    on.handle::<UpdateDownload, _, _>(|params, ctx| async move {
        Ok(UpdateDownloadResult {
            id: ctx.runtime.updates().start(params.id),
        })
    });
}
