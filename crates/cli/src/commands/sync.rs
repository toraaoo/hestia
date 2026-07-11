//! `hestia sync …` — the settings/configs shared across servers and instances.
//! Targets are game-relative paths kept separate per kind (`sync server …` /
//! `sync instance …`); each entry keeps its own copy, reconciled newest-wins
//! with its kind's shared store at every start/launch.

use anyhow::Result;
use clap::Subcommand;
use client::proto::sync::{SyncKind, SyncTargets};

use crate::ui::{self, View};

#[derive(Subcommand)]
pub enum SyncCmd {
    /// The shared store and each kind's targets
    #[command(alias = "list", alias = "ls")]
    Status,
    /// What servers share
    Server {
        #[command(subcommand)]
        cmd: EditCmd,
    },
    /// What instances share
    Instance {
        #[command(subcommand)]
        cmd: EditCmd,
    },
}

#[derive(Subcommand)]
pub enum EditCmd {
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
        SyncCmd::Server { cmd } => edit(SyncKind::Server, "server", cmd).await,
        SyncCmd::Instance { cmd } => edit(SyncKind::Instance, "instance", cmd).await,
    }
}

async fn status() -> Result<()> {
    let client = super::connect().await?;
    let config = client.sync().get().await?;
    ui::show(View::detail([(
        "shared store",
        config.shared_dir.display().to_string(),
    )]))?;
    render(&config.servers, "servers")?;
    render(&config.instances, "instances")
}

fn render(targets: &SyncTargets, noun: &str) -> Result<()> {
    if targets.files.is_empty() && targets.folders.is_empty() {
        return ui::show(View::note(format!("{noun}: no sync targets")));
    }
    ui::show(View::line(format!("{noun}:")))?;
    let mut rows: Vec<Vec<String>> = Vec::new();
    for path in &targets.folders {
        rows.push(vec!["folder".to_string(), path.clone()]);
    }
    for path in &targets.files {
        rows.push(vec!["file".to_string(), path.clone()]);
    }
    ui::show(View::table(noun, ["KIND", "PATH"], rows))
}

async fn edit(kind: SyncKind, noun: &str, cmd: EditCmd) -> Result<()> {
    let client = super::connect().await?;
    let mut targets = current(&client, kind).await?;
    match cmd {
        EditCmd::Add { path, folder } => {
            let added = if folder {
                targets.folders.insert(path.clone())
            } else {
                targets.files.insert(path.clone())
            };
            if !added {
                return ui::show(View::note(format!("{noun}s already share '{path}'")));
            }
            client.sync().set(kind, targets).await?;
            ui::show(View::line(format!("{noun}s now share '{path}'")))
        }
        EditCmd::Remove { path } => {
            let removed = targets.files.remove(&path) || targets.folders.remove(&path);
            if !removed {
                return ui::show(View::note(format!("{noun}s do not share '{path}'")));
            }
            client.sync().set(kind, targets).await?;
            ui::show(View::line(format!("{noun}s no longer share '{path}'")))
        }
    }
}

async fn current(client: &client::Client, kind: SyncKind) -> Result<SyncTargets> {
    let config = client.sync().get().await?;
    Ok(match kind {
        SyncKind::Server => config.servers,
        SyncKind::Instance => config.instances,
    })
}
