//! Server backups: create and restore are jobs over the `BackupManager`, list
//! and remove answer inline. Backups are a server feature — instances have
//! none (import/export is the intended replacement, not yet built).

use proto::backup::{
    BackupJobResult, BackupListResult, ServerBackupCreate, ServerBackupList, ServerBackupRemove,
    ServerBackupRestore,
};
use proto::Empty;

use super::guards::{ensure_no_content, find_server, is_running, require_backup};
use crate::runtime::{server_process_id, BackupJob, Channels, ServiceError};

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<ServerBackupCreate, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        if !record.ready {
            return Err(ServiceError::bad_request(format!(
                "server '{}' is still provisioning",
                record.name
            )));
        }
        if ctx.runtime.server_updates().in_flight(&record.id) {
            return Err(ServiceError::bad_request(format!(
                "server '{}' is being updated",
                record.name
            )));
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
            None => Err(ServiceError::bad_request(
                "a backup or restore is already running for that server",
            )),
        }
    });

    on.handle::<ServerBackupList, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        let backups = ctx
            .runtime
            .engine()
            .server_backups(&record.id)
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(BackupListResult { backups })
    });

    on.handle::<ServerBackupRestore, _, _>(|p, ctx| async move {
        if p.backup.is_empty() {
            return Err(ServiceError::bad_request("backup is required"));
        }
        let record = find_server(&ctx, &p.server)?;
        if is_running(&ctx, &server_process_id(&record.id)) {
            return Err(ServiceError::bad_request(format!(
                "server '{}' is running; stop it first",
                record.name
            )));
        }
        if ctx.runtime.server_updates().in_flight(&record.id) {
            return Err(ServiceError::bad_request(format!(
                "server '{}' is being updated",
                record.name
            )));
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
            None => Err(ServiceError::bad_request(
                "a backup or restore is already running for that server",
            )),
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
            Ok(false) => Err(ServiceError::not_found(format!(
                "no backup matches '{}'",
                p.backup
            ))),
            Err(e) => Err(ServiceError::handler_error(format!("{e:#}"))),
        }
    });
}
