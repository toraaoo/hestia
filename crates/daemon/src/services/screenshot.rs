//! The screenshots across every instance that shares them.

use proto::screenshot::{
    ScreenshotDelete, ScreenshotList, ScreenshotListParams, ScreenshotListResult,
};

use crate::runtime::Channels;

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<ScreenshotList, _, _>(|p: ScreenshotListParams, ctx| async move {
        let screenshots = ctx
            .runtime
            .engine()
            .screenshots(&p.instance)
            .map_err(crate::runtime::engine_error)?;
        Ok(ScreenshotListResult { screenshots })
    });

    on.handle::<ScreenshotDelete, _, _>(|p, ctx| async move {
        ctx.runtime
            .engine()
            .delete_screenshot(&p.instance, &p.file)
            .map_err(crate::runtime::engine_error)?;
        let screenshots = ctx
            .runtime
            .engine()
            .screenshots("")
            .map_err(crate::runtime::engine_error)?;
        Ok(ScreenshotListResult { screenshots })
    });
}
