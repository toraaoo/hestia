//! `hestia daemon …` — daemon lifecycle.

use anyhow::Result;
use clap::Subcommand;

use crate::ui::{self, View};

#[derive(Subcommand)]
pub enum DaemonCmd {
    /// Running (pid, uptime, home, log) or stopped
    Status,
    /// Start the daemon
    Start,
    /// Stop the daemon
    Stop,
    /// Stop then start; picks up a newly built hestiad
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
        DaemonCmd::Stop => match super::connect_running().await {
            Ok(client) => {
                client.daemon().stop().await?;
                ui::show(View::line("hestiad stopping"))?;
            }
            Err(_) => ui::show(View::line("hestiad is not running"))?,
        },
        DaemonCmd::Restart => {
            if let Ok(client) = super::connect_running().await {
                let _ = client.daemon().stop().await;
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
