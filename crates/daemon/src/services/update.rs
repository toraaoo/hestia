//! Self-update: the released-version check, the signed artifact download, and
//! applying it.

use proto::update::{
    UpdateApply, UpdateApplyResult, UpdateCheck, UpdateDownload, UpdateDownloadResult,
};
use proto::Empty;

use crate::runtime::Channels;

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<UpdateCheck, _, _>(|_: Empty, ctx| async move {
        ctx.runtime
            .engine()
            .update()
            .check()
            .await
            .map_err(crate::runtime::engine_error)
    });

    on.handle::<UpdateDownload, _, _>(|params, ctx| async move {
        Ok(UpdateDownloadResult {
            id: ctx.runtime.updates().start(params.id),
        })
    });

    on.handle::<UpdateApply, _, _>(|params, ctx| async move {
        let runtime = ctx.runtime.clone();
        tokio::task::spawn_blocking(move || runtime.engine().update().apply(&params.path))
            .await
            .map_err(|e| crate::runtime::engine_error(anyhow::anyhow!("{e}")))?
            .map(|applied| UpdateApplyResult {
                relaunches: applied.relaunches,
            })
            .map_err(crate::runtime::engine_error)
    });
}
