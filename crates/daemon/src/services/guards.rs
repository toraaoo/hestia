//! Preconditions shared by the domain registrars: resolving an entry by
//! reference, and refusing an operation that would race a running process or an
//! in-flight job.

use proto::error::{EntryKind, ErrorInfo};
use proto::process::ProcessState;

use crate::runtime::HandlerContext;

fn entry_kind(noun: &str) -> EntryKind {
    if noun == "instance" {
        EntryKind::Instance
    } else {
        EntryKind::Server
    }
}

pub(super) fn find_server(
    ctx: &HandlerContext,
    reference: &str,
) -> Result<engine::ServerRecord, ErrorInfo> {
    ctx.runtime
        .engine()
        .servers()
        .get(reference)
        .ok_or_else(|| ErrorInfo::EntryNotFound {
            entry: EntryKind::Server,
            reference: reference.to_string(),
        })
}

pub(super) fn find_instance(
    ctx: &HandlerContext,
    reference: &str,
) -> Result<engine::InstanceRecord, ErrorInfo> {
    ctx.runtime
        .engine()
        .instances()
        .get(reference)
        .ok_or_else(|| ErrorInfo::EntryNotFound {
            entry: EntryKind::Instance,
            reference: reference.to_string(),
        })
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
) -> Result<(), ErrorInfo> {
    if is_running(ctx, process_id) {
        return Err(ErrorInfo::EntryRunning {
            entry: entry_kind(noun),
            name: name.to_string(),
        });
    }
    Ok(())
}

pub(super) fn ensure_no_update(
    ctx: &HandlerContext,
    server_id: &str,
    name: &str,
) -> Result<(), ErrorInfo> {
    if ctx.runtime.server_updates().in_flight(server_id) {
        return Err(ErrorInfo::UpdateInProgress {
            name: name.to_string(),
        });
    }
    Ok(())
}

/// Refuse operations that would race an in-flight content install/update; the
/// entry's process id doubles as the content in-flight key.
pub(super) fn ensure_no_content(
    ctx: &HandlerContext,
    key: &str,
    name: &str,
) -> Result<(), ErrorInfo> {
    if ctx.runtime.content_jobs().in_flight(key) {
        return Err(ErrorInfo::ContentInProgress {
            name: name.to_string(),
        });
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
) -> Result<(), ErrorInfo> {
    if ctx.runtime.backups().in_flight(key) {
        return Err(ErrorInfo::BackupInProgress {
            name: name.to_string(),
        });
    }
    Ok(())
}

pub(super) fn require_content_items(
    spec: &proto::content::ContentAddSpec,
) -> Result<(), ErrorInfo> {
    if spec.items.is_empty() {
        return Err(ErrorInfo::NothingToDo {
            what: proto::error::Task::Install,
        });
    }
    for item in &spec.items {
        let picked = [&item.project, &item.url, &item.path]
            .iter()
            .filter(|s| !s.is_empty())
            .count();
        if picked != 1 {
            return Err(ErrorInfo::MutuallyExclusive {
                options: vec!["a project".into(), "a url".into(), "a file".into()],
            });
        }
    }
    if !spec.worlds.is_empty() && spec.kind != proto::content::ContentKind::DataPack {
        return Err(ErrorInfo::UnsupportedOperation {
            reason: proto::error::Unsupported::WorldsForDatapacksOnly,
        });
    }
    Ok(())
}

pub(super) fn require_backup(
    backups: anyhow::Result<Vec<proto::backup::BackupInfo>>,
    reference: &str,
) -> Result<(), ErrorInfo> {
    let backups = backups.map_err(crate::runtime::internal)?;
    if backups.iter().any(|b| b.id == reference) {
        Ok(())
    } else {
        Err(ErrorInfo::BackupNotFound {
            reference: reference.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use proto::content::{ContentAddItem, ContentAddSpec, ContentKind};

    use super::require_content_items;

    fn project_item(project: &str) -> ContentAddItem {
        ContentAddItem {
            project: project.to_string(),
            ..ContentAddItem::default()
        }
    }

    #[test]
    fn accepts_a_batch_of_single_selector_items() {
        let spec = ContentAddSpec {
            items: vec![project_item("sodium"), project_item("lithium")],
            ..ContentAddSpec::default()
        };
        assert!(require_content_items(&spec).is_ok());
    }

    #[test]
    fn rejects_an_empty_batch() {
        assert!(require_content_items(&ContentAddSpec::default()).is_err());
    }

    #[test]
    fn rejects_an_item_with_no_or_multiple_selectors() {
        let empty = ContentAddSpec {
            items: vec![ContentAddItem::default()],
            ..ContentAddSpec::default()
        };
        assert!(require_content_items(&empty).is_err());

        let mut both = project_item("sodium");
        both.url = "https://modrinth.com/mod/sodium".to_string();
        let spec = ContentAddSpec {
            items: vec![both],
            ..ContentAddSpec::default()
        };
        assert!(require_content_items(&spec).is_err());
    }

    #[test]
    fn rejects_worlds_on_non_datapack_kinds() {
        let spec = ContentAddSpec {
            kind: ContentKind::Mod,
            items: vec![project_item("sodium")],
            worlds: vec!["world".to_string()],
            ..ContentAddSpec::default()
        };
        assert!(require_content_items(&spec).is_err());

        let spec = ContentAddSpec {
            kind: ContentKind::DataPack,
            items: vec![project_item("terralith")],
            worlds: vec!["world".to_string()],
            ..ContentAddSpec::default()
        };
        assert!(require_content_items(&spec).is_ok());
    }
}
