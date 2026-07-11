//! Shared settings/configs: the per-kind sets of game-relative files/folders
//! propagated across entries. Apply runs inside each entry's launch flow; these
//! channels only read and edit the target sets.

use proto::sync::{SyncConfig, SyncGet, SyncKind, SyncSet, SyncSetParams};
use proto::Empty;

use crate::runtime::{Channels, ServiceError};

fn config(sync: &engine::Sync) -> SyncConfig {
    SyncConfig {
        shared_dir: sync.dir(),
        servers: sync.targets(SyncKind::Server),
        instances: sync.targets(SyncKind::Instance),
    }
}

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<SyncGet, _, _>(
        |_: Empty, ctx| async move { Ok(config(ctx.runtime.engine().sync())) },
    );

    on.handle::<SyncSet, _, _>(|p: SyncSetParams, ctx| async move {
        let sync = ctx.runtime.engine().sync();
        let targets = sync
            .set_targets(p.kind, p.targets)
            .map_err(|e| ServiceError::bad_request(format!("{e:#}")))?;
        tracing::info!(
            kind = ?p.kind,
            files = targets.files.len(),
            folders = targets.folders.len(),
            "sync targets updated"
        );
        Ok(config(sync))
    });
}
