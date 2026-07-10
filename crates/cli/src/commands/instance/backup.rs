//! `hestia instance backup create|list|restore|remove` — the per-instance
//! backup surface. Create and restore need the instance stopped, render live
//! progress, and restore confirms before replacing the current data. Instances
//! back up on demand only — no schedule.

use std::sync::Arc;

use anyhow::Result;
use clap::Subcommand;
use client::Client;

use super::entry;
use crate::commands::mc;
use crate::ui::{self, ProvisionReporter, View};

#[derive(Subcommand)]
pub enum BackupCmd {
    /// Archive a stopped instance's game directory
    Create {
        /// Instance name or id (prompts when omitted)
        instance: Option<String>,
    },
    /// Stored backups, newest first
    #[command(visible_alias = "ls")]
    List {
        /// Instance name or id (prompts when omitted)
        instance: Option<String>,
    },
    /// Replace a stopped instance's game directory with a backup's content
    Restore {
        /// Instance name or id (prompts when omitted)
        instance: Option<String>,
        /// Backup id (prompts when omitted)
        backup: Option<String>,
        #[arg(long, help = "Replace the current data without confirming")]
        force: bool,
    },
    /// Delete a backup
    #[command(visible_alias = "rm")]
    Remove {
        /// Instance name or id (prompts when omitted)
        instance: Option<String>,
        /// Backup id (prompts when omitted)
        backup: Option<String>,
    },
}

pub(super) async fn run(client: &Client, cmd: BackupCmd) -> Result<()> {
    match cmd {
        BackupCmd::Create { instance } => {
            let info = entry::pick_instance(client.instance().list().await?, instance)?;
            let reporter = Arc::new(ProvisionReporter::new());
            reporter.update(&mc::backup_phase());
            let progress = reporter.clone();
            let result = client
                .instance()
                .backup_create(&info.id, move |p| progress.update(p))
                .await;
            reporter.finish();
            let backup = result?;
            ui::show(View::line(format!(
                "backup '{}' of '{}' created ({})",
                backup.id,
                info.name,
                ui::human_bytes(backup.size)
            )))
        }
        BackupCmd::List { instance } => {
            let info = entry::pick_instance(client.instance().list().await?, instance)?;
            let backups = client.instance().backup_list(&info.id).await?;
            if backups.is_empty() {
                return ui::show(View::note("no backups yet (hestia instance backup create)"));
            }
            mc::show_backups(format!("{} backups", info.name), backups)
        }
        BackupCmd::Restore {
            instance,
            backup,
            force,
        } => {
            let info = entry::pick_instance(client.instance().list().await?, instance)?;
            let backups = client.instance().backup_list(&info.id).await?;
            let backup = mc::pick_backup(backups, backup)?;
            if !force {
                mc::confirm_restore(&info.name, "saves and settings", &backup)?;
            }
            let reporter = Arc::new(ProvisionReporter::new());
            reporter.update(&mc::backup_phase());
            let progress = reporter.clone();
            let result = client
                .instance()
                .backup_restore(&info.id, &backup.id, move |p| progress.update(p))
                .await;
            reporter.finish();
            result?;
            ui::show(View::line(format!(
                "backup '{}' restored onto '{}'",
                backup.id, info.name
            )))
        }
        BackupCmd::Remove { instance, backup } => {
            let info = entry::pick_instance(client.instance().list().await?, instance)?;
            let backups = client.instance().backup_list(&info.id).await?;
            let backup = mc::pick_backup(backups, backup)?;
            client
                .instance()
                .backup_remove(&info.id, &backup.id)
                .await?;
            ui::show(View::line(format!("backup '{}' removed", backup.id)))
        }
    }
}
