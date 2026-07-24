//! Ad-hoc downloads driven off-thread by the download manager.

use proto::download::DownloadStart;
use proto::error::{ErrorInfo, Field};

use crate::runtime::Channels;

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<DownloadStart, _, _>(|spec, ctx| async move {
        if spec.url.is_empty() {
            return Err(ErrorInfo::FieldRequired { field: Field::Url });
        }
        let id = ctx.runtime.downloads().start(spec);
        Ok(proto::download::DownloadStartResult { id })
    });
}
