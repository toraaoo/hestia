//! Preconditions shared by the domain registrars: resolving an entry by
//! reference, and refusing an operation that would race a running process or an
//! in-flight job.

use proto::process::ProcessState;

use crate::runtime::{HandlerContext, ServiceError};

pub(super) fn find_server(
    ctx: &HandlerContext,
    reference: &str,
) -> Result<engine::ServerRecord, ServiceError> {
    ctx.runtime
        .engine()
        .servers()
        .get(reference)
        .ok_or_else(|| ServiceError::not_found(format!("no server matches '{reference}'")))
}

pub(super) fn find_instance(
    ctx: &HandlerContext,
    reference: &str,
) -> Result<engine::InstanceRecord, ServiceError> {
    ctx.runtime
        .engine()
        .instances()
        .get(reference)
        .ok_or_else(|| ServiceError::not_found(format!("no instance matches '{reference}'")))
}

pub(super) fn is_running(ctx: &HandlerContext, process_id: &str) -> bool {
    ctx.runtime
        .processes()
        .status(process_id)
        .is_some_and(|info| info.state == ProcessState::Running)
}

/// Refuse content changes on a running entry: the JVM holds its jars open
/// (locked on Windows), and changes only apply at the next start anyway.
pub(super) fn ensure_stopped(
    ctx: &HandlerContext,
    process_id: &str,
    noun: &str,
    name: &str,
) -> Result<(), ServiceError> {
    if is_running(ctx, process_id) {
        return Err(ServiceError::bad_request(format!(
            "{noun} '{name}' is running; stop it first"
        )));
    }
    Ok(())
}

pub(super) fn ensure_no_update(
    ctx: &HandlerContext,
    server_id: &str,
    name: &str,
) -> Result<(), ServiceError> {
    if ctx.runtime.server_updates().in_flight(server_id) {
        return Err(ServiceError::bad_request(format!(
            "server '{name}' is being updated"
        )));
    }
    Ok(())
}

/// Refuse operations that would race an in-flight content install/update; the
/// entry's process id doubles as the content in-flight key.
pub(super) fn ensure_no_content(
    ctx: &HandlerContext,
    key: &str,
    name: &str,
) -> Result<(), ServiceError> {
    if ctx.runtime.content_jobs().in_flight(key) {
        return Err(ServiceError::bad_request(format!(
            "'{name}' has a content change in progress; wait for it to finish"
        )));
    }
    Ok(())
}

/// Refuse lifecycle changes (start, update, remove) while an archive is being
/// written or restored — they would race the file tree it is reading. The
/// entry's process id doubles as the backup in-flight key.
pub(super) fn ensure_no_backup(
    ctx: &HandlerContext,
    key: &str,
    name: &str,
) -> Result<(), ServiceError> {
    if ctx.runtime.backups().in_flight(key) {
        return Err(ServiceError::bad_request(format!(
            "'{name}' has a backup or restore in progress; wait for it to finish"
        )));
    }
    Ok(())
}

pub(super) fn require_one_content_source(
    spec: &proto::content::ContentAddSpec,
) -> Result<(), ServiceError> {
    let picked = [&spec.project, &spec.url, &spec.path]
        .iter()
        .filter(|s| !s.is_empty())
        .count();
    if picked != 1 {
        return Err(ServiceError::bad_request(
            "specify exactly one of a project, a url, or a file",
        ));
    }
    Ok(())
}

pub(super) fn require_backup(
    backups: anyhow::Result<Vec<proto::backup::BackupInfo>>,
    reference: &str,
) -> Result<(), ServiceError> {
    let backups = backups.map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
    if backups.iter().any(|b| b.id == reference) {
        Ok(())
    } else {
        Err(ServiceError::not_found(format!(
            "no backup matches '{reference}'"
        )))
    }
}
