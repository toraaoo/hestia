//! `hestia sync …` — the settings/configs shared across instances. Targets are
//! game-relative paths; each instance keeps its own copy, reconciled newest-wins
//! with the shared store at every launch.

use anyhow::Result;
use clap::Subcommand;
use client::proto::sync::SyncTargets;

use crate::ui::{self, View};

#[derive(Subcommand)]
pub enum SyncCmd {
    /// The shared store and its targets
    #[command(alias = "list", alias = "ls")]
    Status,
    /// Share a file (or a whole folder with `--folder`)
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
    match cmd {
        SyncCmd::Status => status().await,
        SyncCmd::Add { path, folder } => add(path, folder).await,
        SyncCmd::Remove { path } => remove(path).await,
    }
}

async fn status() -> Result<()> {
    let client = super::connect().await?;
    let config = client.sync().get().await?;
    ui::show(View::detail([(
        "shared store",
        config.shared_dir.display().to_string(),
    )]))?;
    render(&config.targets)
}

fn render(targets: &SyncTargets) -> Result<()> {
    if targets.files.is_empty() && targets.folders.is_empty() {
        return ui::show(View::note("no sync targets"));
    }
    let mut rows: Vec<Vec<String>> = Vec::new();
    for path in &targets.folders {
        rows.push(vec!["folder".to_string(), path.clone()]);
    }
    for path in &targets.files {
        rows.push(vec!["file".to_string(), path.clone()]);
    }
    ui::show(View::table("Sync targets", ["KIND", "PATH"], rows))
}

async fn add(path: String, folder: bool) -> Result<()> {
    let client = super::connect().await?;
    let mut targets = client.sync().get().await?.targets;
    let added = if folder {
        targets.folders.insert(path.clone())
    } else {
        targets.files.insert(path.clone())
    };
    if !added {
        return ui::show(View::note(format!("instances already share '{path}'")));
    }
    client.sync().set(targets).await?;
    ui::show(View::line(format!("instances now share '{path}'")))
}

async fn remove(path: String) -> Result<()> {
    let client = super::connect().await?;
    let mut targets = client.sync().get().await?.targets;
    let removed = targets.files.remove(&path) || targets.folders.remove(&path);
    if !removed {
        return ui::show(View::note(format!("instances do not share '{path}'")));
    }
    client.sync().set(targets).await?;
    ui::show(View::line(format!("instances no longer share '{path}'")))
}
