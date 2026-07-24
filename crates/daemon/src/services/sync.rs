//! Shared settings/configs: the set of game-relative files (copied) and
//! folders (linked) propagated across instances. Apply runs inside the
//! instance launch flow; these channels read and edit the target set, report
//! each instance's per-target link state, and run the adopt migration.

use proto::error::{EntryKind, ErrorInfo};
use proto::sync::{
    SyncAdopt, SyncAdoptResult, SyncConfig, SyncGet, SyncSet, SyncSetParams, SyncStatus,
    SyncStatusResult,
};
use proto::Empty;

use super::guards::{ensure_no_content, find_instance};
use crate::runtime::{instance_process_id, Channels};

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
            .map_err(crate::runtime::engine_error)?;
        tracing::info!(
            files = targets.files.len(),
            folders = targets.folders.len(),
            "sync targets updated"
        );
        Ok(config(sync))
    });

    on.handle::<SyncStatus, _, _>(|_: Empty, ctx| async move {
        Ok(SyncStatusResult {
            instances: ctx.runtime.engine().sync_status(),
        })
    });

    on.handle::<SyncAdopt, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        if ctx.runtime.instance_running(&record.id) {
            return Err(ErrorInfo::EntryRunning {
                entry: EntryKind::Instance,
                name: record.name.clone(),
            });
        }
        ensure_no_content(&ctx, &instance_process_id(&record.id), &record.name)?;
        let adopted = ctx
            .runtime
            .engine()
            .adopt_instance_sync(&record.id, &p.targets)
            .map_err(crate::runtime::engine_error)?;
        tracing::info!(
            instance = %record.id,
            targets = adopted.len(),
            "sync folders adopted into the shared store"
        );
        Ok(SyncAdoptResult { adopted })
    });
}
