//! Server backups: create and restore are jobs over the `BackupManager`, list
//! and remove answer inline. Backups are a server feature — instances have
//! none (import/export is the intended replacement, not yet built).

use proto::backup::{
    BackupJobResult, BackupListResult, ServerBackupCreate, ServerBackupList, ServerBackupRemove,
    ServerBackupRestore,
};
use proto::error::ErrorInfo;
use proto::Empty;

use super::guards::{ensure_no_content, find_server, is_running, require_backup};
use crate::runtime::{server_process_id, BackupJob, Channels};

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<ServerBackupCreate, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        if !record.ready {
            return Err(ErrorInfo::Provisioning {
                name: record.name.clone(),
            });
        }
        if ctx.runtime.server_updates().in_flight(&record.id) {
            return Err(ErrorInfo::UpdateInProgress {
                name: record.name.clone(),
            });
        }
        let live = is_running(&ctx, &server_process_id(&record.id));
        match ctx.runtime.backups().start(
            BackupJob::ServerBackup {
                server_id: record.id,
                live,
            },
            p.id,
        ) {
            Some(id) => Ok(BackupJobResult { id }),
            None => Err(ErrorInfo::BackupInProgress {
                name: record.name.clone(),
            }),
        }
    });

    on.handle::<ServerBackupList, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        let backups = ctx
            .runtime
            .engine()
            .server_backups(&record.id)
            .map_err(crate::runtime::internal)?;
        Ok(BackupListResult { backups })
    });

    on.handle::<ServerBackupRestore, _, _>(|p, ctx| async move {
        if p.backup.is_empty() {
            return Err(ErrorInfo::FieldRequired {
                field: proto::error::Field::Backup,
            });
        }
        let record = find_server(&ctx, &p.server)?;
        if is_running(&ctx, &server_process_id(&record.id)) {
            return Err(ErrorInfo::EntryRunning {
                entry: proto::error::EntryKind::Server,
                name: record.name.clone(),
            });
        }
        if ctx.runtime.server_updates().in_flight(&record.id) {
            return Err(ErrorInfo::UpdateInProgress {
                name: record.name.clone(),
            });
        }
        ensure_no_content(&ctx, &server_process_id(&record.id), &record.name)?;
        require_backup(ctx.runtime.engine().server_backups(&record.id), &p.backup)?;
        match ctx.runtime.backups().start(
            BackupJob::ServerRestore {
                server_id: record.id,
                backup: p.backup,
            },
            p.id,
        ) {
            Some(id) => Ok(BackupJobResult { id }),
            None => Err(ErrorInfo::BackupInProgress {
                name: record.name.clone(),
            }),
        }
    });

    on.handle::<ServerBackupRemove, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        match ctx
            .runtime
            .engine()
            .remove_server_backup(&record.id, &p.backup)
        {
            Ok(true) => Ok(Empty {}),
            Ok(false) => Err(ErrorInfo::BackupNotFound {
                reference: p.backup.clone(),
            }),
            Err(e) => Err(crate::runtime::internal(e)),
        }
    });
}
