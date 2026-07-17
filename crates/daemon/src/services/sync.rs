//! Shared settings/configs: the set of game-relative files/folders propagated
//! across instances. Apply runs inside the instance launch flow; these channels
//! only read and edit the target set.

use proto::sync::{SyncConfig, SyncGet, SyncSet, SyncSetParams};
use proto::Empty;

use crate::runtime::{Channels, ServiceError};

fn config(sync: &engine::Sync) -> SyncConfig {
    SyncConfig {
        shared_dir: sync.dir(),
        targets: sync.targets(),
    }
}

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<SyncGet, _, _>(
        |_: Empty, ctx| async move { Ok(config(ctx.runtime.engine().sync())) },
    );

    on.handle::<SyncSet, _, _>(|p: SyncSetParams, ctx| async move {
        let sync = ctx.runtime.engine().sync();
        let targets = sync
            .set_targets(p.targets)
            .map_err(|e| ServiceError::bad_request(format!("{e:#}")))?;
        tracing::info!(
            files = targets.files.len(),
            folders = targets.folders.len(),
            "sync targets updated"
        );
        Ok(config(sync))
    });
}
