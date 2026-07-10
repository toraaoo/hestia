//! Ad-hoc downloads driven off-thread by the download manager.

use proto::download::DownloadStart;

use crate::runtime::{Channels, ServiceError};

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<DownloadStart, _, _>(|spec, ctx| async move {
        if spec.url.is_empty() {
            return Err(ServiceError::bad_request("download url is empty"));
        }
        let id = ctx.runtime.downloads().start(spec);
        Ok(proto::download::DownloadStartResult { id })
    });
}
