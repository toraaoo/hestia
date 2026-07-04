//! `hestia java …` — release lines, install/uninstall, and installed runtimes.

use std::io::Write;

use anyhow::Result;
use clap::Subcommand;
use client::proto::java::{JavaInstallPhase, JavaInstallProgress};

use crate::output::{human_bytes, print_table};

#[derive(Subcommand)]
pub enum JavaCmd {
    /// Release lines the provider ships
    Available,
    /// Resolve, download, verify, extract, and register a runtime
    Install {
        major: i32,
        #[arg(long, help = "Reinstall even if the line is already present")]
        force: bool,
    },
    /// Installed runtimes
    List,
    /// Remove an installed runtime
    Uninstall { major: i32 },
}

pub async fn run(cmd: JavaCmd) -> Result<()> {
    let client = super::connect().await?;
    match cmd {
        JavaCmd::Available => {
            let releases = client.java().releases().await?;
            let rows = releases
                .iter()
                .map(|r| {
                    vec![
                        r.major.to_string(),
                        if r.lts { "LTS".into() } else { String::new() },
                    ]
                })
                .collect::<Vec<_>>();
            print_table(&["MAJOR", "TYPE"], &rows);
        }
        JavaCmd::List => {
            let runtimes = client.java().list().await?;
            if runtimes.is_empty() {
                println!("no java runtimes installed");
                return Ok(());
            }
            let rows = runtimes
                .iter()
                .map(|r| {
                    vec![
                        r.major.to_string(),
                        r.vendor.clone(),
                        r.release_name.clone(),
                        r.home.display().to_string(),
                    ]
                })
                .collect::<Vec<_>>();
            print_table(&["MAJOR", "VENDOR", "RELEASE", "HOME"], &rows);
        }
        JavaCmd::Install { major, force } => {
            let (runtime, already) = client.java().install(major, force, print_progress).await?;
            eprintln!();
            if already {
                println!(
                    "java {} already installed ({})",
                    runtime.major, runtime.release_name
                );
            } else {
                println!(
                    "installed java {} ({})",
                    runtime.major, runtime.release_name
                );
            }
        }
        JavaCmd::Uninstall { major } => {
            client.java().uninstall(major).await?;
            println!("uninstalled java {major}");
        }
    }
    Ok(())
}

fn print_progress(p: &JavaInstallProgress) {
    match p.phase {
        JavaInstallPhase::Resolving => eprint!("\rresolving…                    "),
        JavaInstallPhase::Downloading => {
            let total = if p.total > 0 {
                human_bytes(p.total)
            } else {
                "?".into()
            };
            eprint!(
                "\rdownloading {} / {}            ",
                human_bytes(p.current),
                total
            );
        }
        JavaInstallPhase::Extracting => eprint!("\rextracting…                   "),
    }
    let _ = std::io::stderr().flush();
}
