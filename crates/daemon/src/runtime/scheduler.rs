//! The scheduled-backup loop: every minute, archive each *running* server
//! whose `backup-interval` has elapsed since its newest backup (of any kind —
//! a fresh manual or pre-update archive resets the clock), then prune its
//! scheduled archives beyond `backup-retention`. A stopped server's world
//! cannot change, so it is never re-archived on schedule; the pre-update and
//! on-demand backups cover it.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use proto::backup::BackupKind;
use proto::process::ProcessState;

use super::{server_process_id, Runtime};

const TICK: Duration = Duration::from_secs(60);

pub fn spawn_backup_scheduler(runtime: Arc<Runtime>) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            run_due_backups(&runtime).await;
        }
    });
}

async fn run_due_backups(runtime: &Runtime) {
    let engine = runtime.engine();
    for record in engine.servers().list() {
        let Some(interval) = record.backup.interval() else {
            continue;
        };
        if !record.ready {
            continue;
        }
        let running = runtime
            .processes()
            .status(&server_process_id(&record.id))
            .is_some_and(|p| p.state == ProcessState::Running);
        if !running {
            continue;
        }
        let newest = engine
            .server_backups(&record.id)
            .ok()
            .and_then(|backups| backups.first().map(|b| b.created_unix));
        let due = newest.is_none_or(|t| now_unix().saturating_sub(t) >= interval.as_secs() as i64);
        if !due {
            continue;
        }

        tracing::info!(server = %record.id, "scheduled backup starting");
        match engine
            .backup_server(&record.id, BackupKind::Scheduled, true, &|_| {})
            .await
        {
            Ok(backup) => {
                tracing::info!(
                    server = %record.id,
                    backup = %backup.id,
                    size = backup.size,
                    "scheduled backup done"
                );
                match engine.prune_server_backups(&record.id) {
                    Ok(pruned) if !pruned.is_empty() => {
                        tracing::info!(server = %record.id, pruned = pruned.len(), "old scheduled backups pruned");
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(server = %record.id, error = format!("{e:#}"), "backup prune failed");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(server = %record.id, error = format!("{e:#}"), "scheduled backup failed");
            }
        }
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
