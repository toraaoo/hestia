//! `hestia sync …` — the settings/configs shared across servers and instances.
//! Targets are game-relative paths; each entry keeps its own copy, reconciled
//! newest-wins with the shared store at every start/launch.

use anyhow::Result;
use clap::Subcommand;

use crate::ui::{self, View};

#[derive(Subcommand)]
pub enum SyncCmd {
    /// The shared store and its current targets
    #[command(alias = "list", alias = "ls")]
    Status,
    /// Share a file (or a whole folder with `--folder`) across entries
    Add {
        /// Game-relative path, e.g. `options.txt` or `config`
        path: String,
        /// Treat the path as a folder (sync every file under it)
        #[arg(long)]
        folder: bool,
    },
    /// Stop sharing a target
    #[command(alias = "rm")]
    Remove {
        /// The file or folder path to remove
        path: String,
    },
}

pub async fn run(cmd: SyncCmd) -> Result<()> {
    let client = super::connect().await?;
    match cmd {
        SyncCmd::Status => {
            let config = client.sync().get().await?;
            ui::show(View::detail([(
                "shared store",
                config.shared_dir.display().to_string(),
            )]))?;
            if config.targets.files.is_empty() && config.targets.folders.is_empty() {
                return ui::show(View::note("no sync targets"));
            }
            let mut rows: Vec<Vec<String>> = Vec::new();
            for path in &config.targets.folders {
                rows.push(vec!["folder".to_string(), path.clone()]);
            }
            for path in &config.targets.files {
                rows.push(vec!["file".to_string(), path.clone()]);
            }
            ui::show(View::table("sync targets", ["KIND", "PATH"], rows))
        }
        SyncCmd::Add { path, folder } => {
            let mut targets = client.sync().get().await?.targets;
            let added = if folder {
                targets.folders.insert(path.clone())
            } else {
                targets.files.insert(path.clone())
            };
            if !added {
                return ui::show(View::note(format!("'{path}' is already a sync target")));
            }
            client.sync().set(targets).await?;
            ui::show(View::line(format!("sharing '{path}'")))
        }
        SyncCmd::Remove { path } => {
            let mut targets = client.sync().get().await?.targets;
            let removed = targets.files.remove(&path) || targets.folders.remove(&path);
            if !removed {
                return ui::show(View::note(format!("'{path}' is not a sync target")));
            }
            client.sync().set(targets).await?;
            ui::show(View::line(format!("stopped sharing '{path}'")))
        }
    }
}
