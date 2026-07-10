//! `hestia server backup create|list|restore|remove` — the per-server backup
//! surface. Create and restore render live progress; restore confirms before
//! replacing the current data. Scheduled backups are configured through
//! `server config` (`backup-interval`, `backup-retention`).

use std::sync::Arc;

use anyhow::Result;
use clap::Subcommand;
use client::Client;

use super::entry;
use crate::commands::mc;
use crate::ui::{self, ProvisionReporter, View};

#[derive(Subcommand)]
pub enum BackupCmd {
    /// Archive the server's data (a running server keeps running; its world
    /// saving pauses during the archive)
    Create {
        /// Server name or id (prompts when omitted)
        server: Option<String>,
    },
    /// Stored backups, newest first
    #[command(visible_alias = "ls")]
    List {
        /// Server name or id (prompts when omitted)
        server: Option<String>,
    },
    /// Replace a stopped server's data with a backup's content (the current
    /// jar and libraries are kept)
    Restore {
        /// Server name or id (prompts when omitted)
        server: Option<String>,
        /// Backup id (prompts when omitted)
        backup: Option<String>,
        #[arg(long, help = "Replace the current data without confirming")]
        force: bool,
    },
    /// Delete a backup
    #[command(visible_alias = "rm")]
    Remove {
        /// Server name or id (prompts when omitted)
        server: Option<String>,
        /// Backup id (prompts when omitted)
        backup: Option<String>,
    },
}

pub(super) async fn run(client: &Client, cmd: BackupCmd) -> Result<()> {
    match cmd {
        BackupCmd::Create { server } => {
            let info = entry::pick_server(client.server().list().await?, server)?;
            let reporter = Arc::new(ProvisionReporter::new());
            reporter.update(&mc::backup_phase());
            let progress = reporter.clone();
            let result = client
                .server()
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
        BackupCmd::List { server } => {
            let info = entry::pick_server(client.server().list().await?, server)?;
            let backups = client.server().backup_list(&info.id).await?;
            if backups.is_empty() {
                return ui::show(View::note("no backups yet (hestia server backup create)"));
            }
            mc::show_backups(format!("{} backups", info.name), backups)
        }
        BackupCmd::Restore {
            server,
            backup,
            force,
        } => {
            let info = entry::pick_server(client.server().list().await?, server)?;
            let backups = client.server().backup_list(&info.id).await?;
            let backup = mc::pick_backup(backups, backup)?;
            if !force {
                mc::confirm_restore(&info.name, "world and settings", &backup)?;
            }
            let reporter = Arc::new(ProvisionReporter::new());
            reporter.update(&mc::backup_phase());
            let progress = reporter.clone();
            let result = client
                .server()
                .backup_restore(&info.id, &backup.id, move |p| progress.update(p))
                .await;
            reporter.finish();
            result?;
            ui::show(View::line(format!(
                "backup '{}' restored onto '{}'",
                backup.id, info.name
            )))
        }
        BackupCmd::Remove { server, backup } => {
            let info = entry::pick_server(client.server().list().await?, server)?;
            let backups = client.server().backup_list(&info.id).await?;
            let backup = mc::pick_backup(backups, backup)?;
            client.server().backup_remove(&info.id, &backup.id).await?;
            ui::show(View::line(format!("backup '{}' removed", backup.id)))
        }
    }
}
