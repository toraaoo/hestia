//! `hestia sync …` — the settings/configs shared across instances. File
//! targets are copied newest-wins; folder targets are linked into the shared
//! store, so every instance opens the same physical folders (worlds included).

use anyhow::Result;
use clap::Subcommand;
use client::proto::sync::{InstanceSyncStatus, LinkState, SyncTargets};

use crate::ui::{self, View};

#[derive(Subcommand)]
pub enum SyncCmd {
    /// The shared store, its targets, and each instance's link state
    #[command(alias = "list", alias = "ls")]
    Status,
    /// Share a file (or a whole folder with `--folder`)
    Add {
        /// Game-relative path, e.g. `options.txt` or `config`
        path: String,
        /// Treat the path as a folder (linked into the shared store)
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
    render_targets(&config.targets)?;
    render_status(client.sync().status().await?)
}

fn render_targets(targets: &SyncTargets) -> Result<()> {
    if targets.files.is_empty() && targets.folders.is_empty() {
        return ui::show(View::note("no sync targets"));
    }
    let mut rows: Vec<Vec<String>> = Vec::new();
    for path in &targets.folders {
        rows.push(vec!["folder (linked)".to_string(), path.clone()]);
    }
    for path in &targets.files {
        rows.push(vec!["file (copied)".to_string(), path.clone()]);
    }
    ui::show(View::table("Sync targets", ["CLASS", "PATH"], rows))
}

fn render_status(instances: Vec<InstanceSyncStatus>) -> Result<()> {
    if instances.is_empty() {
        return Ok(());
    }
    let mut blocked = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    for instance in instances {
        for target in instance.targets {
            if target.state == LinkState::CannotLink {
                blocked.push((instance.name.clone(), target.target.clone()));
            }
            rows.push(vec![
                instance.name.clone(),
                target.target,
                state_label(target.state).to_string(),
            ]);
        }
    }
    ui::show(View::table(
        "Link state",
        ["INSTANCE", "TARGET", "STATE"],
        rows,
    ))?;
    for (name, target) in blocked {
        ui::show(View::note(format!(
            "'{name}' has an existing '{target}' — move it into the store with \
             `hestia instance {name} sync adopt {target}`"
        )))?;
    }
    Ok(())
}

fn state_label(state: LinkState) -> &'static str {
    match state {
        LinkState::Linked => "linked",
        LinkState::Pending => "links at next launch",
        LinkState::CannotLink => "cannot link",
    }
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
