//! `hestia daemon …` — daemon lifecycle.

use anyhow::{bail, Result};
use clap::Subcommand;
use client::proto::process::ProcessState;
use client::Client;

use crate::ui::{self, View};

#[derive(Subcommand)]
pub enum DaemonCmd {
    /// Running (pid, uptime, home, log) or stopped
    Status,
    /// Start the daemon
    Start,
    /// Stop the daemon; supervised processes keep running unless --all
    Stop {
        /// Also stop every supervised process (servers, instances)
        #[arg(long, conflicts_with = "keep")]
        all: bool,
        /// Leave supervised processes running
        #[arg(long)]
        keep: bool,
    },
    /// Stop then start; picks up a newly built hestiad. Supervised processes
    /// keep running and are re-adopted
    Restart,
}

pub async fn run(cmd: DaemonCmd) -> Result<()> {
    match cmd {
        DaemonCmd::Status => match super::connect_running().await {
            Ok(client) => {
                let s = client.daemon().status().await?;
                ui::show(View::line("running"))?;
                ui::show(View::detail([
                    ("pid", s.pid.to_string()),
                    ("uptime", format!("{}s", s.uptime_seconds)),
                    ("home", s.home.display().to_string()),
                    ("log", s.log.display().to_string()),
                ]))?;
            }
            Err(_) => ui::show(View::line("stopped"))?,
        },
        DaemonCmd::Start => {
            let client = super::connect().await?;
            let info = client.app().info().await?;
            ui::show(View::line(format!(
                "hestiad running ({} {})",
                info.name, info.version
            )))?;
        }
        DaemonCmd::Stop { all, keep } => match super::connect_running().await {
            Ok(client) => {
                let (stop_processes, running) = stop_choice(&client, all, keep).await?;
                client.daemon().stop(stop_processes).await?;
                ui::show(View::line("hestiad stopping"))?;
                if !stop_processes && !running.is_empty() {
                    let verb = if running.len() == 1 { "keeps" } else { "keep" };
                    ui::show(View::note(format!(
                        "{} {verb} running",
                        summarize(&running)
                    )))?;
                }
            }
            Err(_) => ui::show(View::line("hestiad is not running"))?,
        },
        DaemonCmd::Restart => {
            if let Ok(client) = super::connect_running().await {
                let _ = client.daemon().stop(false).await;
                drop(client);
                // Give the old daemon a moment to release the endpoint.
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            let client = super::connect().await?;
            let info = client.app().info().await?;
            ui::show(View::line(format!(
                "hestiad restarted ({} {})",
                info.name, info.version
            )))?;
        }
    }
    Ok(())
}

async fn stop_choice(client: &Client, all: bool, keep: bool) -> Result<(bool, Vec<String>)> {
    if all {
        return Ok((true, Vec::new()));
    }
    let running = running_workloads(client).await?;
    if keep || running.is_empty() {
        return Ok((false, running));
    }
    if !ui::is_interactive() {
        bail!(
            "{} still running: {}; pass --all to stop them too or --keep to leave them running",
            summarize(&running),
            running.join(", ")
        );
    }
    let stop = ui::prompt_confirm(&format!(
        "{} still running ({}) — stop them too?",
        summarize(&running),
        running.join(", ")
    ))?;
    Ok((stop, running))
}

async fn running_workloads(client: &Client) -> Result<Vec<String>> {
    let processes = client.process().list().await?;
    let mut running: Vec<String> = processes
        .into_iter()
        .filter(|p| p.state == ProcessState::Running)
        .map(|p| p.id)
        .collect();
    if running.is_empty() {
        return Ok(running);
    }
    let servers = client.server().list().await.unwrap_or_default();
    let instances = client.instance().list().await.unwrap_or_default();
    for id in &mut running {
        if let Some(sid) = id.strip_prefix("server-") {
            if let Some(s) = servers.iter().find(|s| s.id == sid) {
                *id = format!("server \"{}\"", s.name);
            }
        } else if let Some(iid) = id.strip_prefix("instance-") {
            if let Some(i) = instances.iter().find(|i| i.id == iid) {
                *id = format!("instance \"{}\"", i.name);
            }
        }
    }
    Ok(running)
}

fn summarize(running: &[String]) -> String {
    if running.len() == 1 {
        "1 supervised process".to_string()
    } else {
        format!("{} supervised processes", running.len())
    }
}
