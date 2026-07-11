//! Shared settings/configs: the global set of game-relative files/folders
//! propagated across entries. Apply runs inside each entry's launch flow; these
//! channels only read and edit the target set.

use proto::sync::{SyncConfig, SyncGet, SyncSet, SyncTargets};
use proto::Empty;

use crate::runtime::{Channels, ServiceError};

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<SyncGet, _, _>(|_: Empty, ctx| async move {
        let sync = ctx.runtime.engine().sync();
        Ok(SyncConfig {
            shared_dir: sync.dir(),
            targets: sync.targets(),
        })
    });

    on.handle::<SyncSet, _, _>(|targets: SyncTargets, ctx| async move {
        let sync = ctx.runtime.engine().sync();
        let targets = sync
            .set_targets(targets)
            .map_err(|e| ServiceError::bad_request(format!("{e:#}")))?;
        tracing::info!(
            files = targets.files.len(),
            folders = targets.folders.len(),
            "sync targets updated"
        );
        Ok(SyncConfig {
            shared_dir: sync.dir(),
            targets,
        })
    });
}
