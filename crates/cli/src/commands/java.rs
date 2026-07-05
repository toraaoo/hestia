//! `hestia java …` — release lines, install/uninstall, and installed runtimes.

use std::sync::Arc;

use anyhow::Result;
use clap::Subcommand;

use crate::ui::{self, InstallReporter, Spinner, View};

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
            let releases = {
                let _spinner = Spinner::start("fetching release lines");
                client.java().releases().await?
            };
            let rows = releases
                .iter()
                .map(|r| {
                    vec![
                        r.major.to_string(),
                        if r.lts { "LTS".into() } else { String::new() },
                    ]
                })
                .collect();
            ui::show(View::table("java", ["MAJOR", "TYPE"], rows))?;
        }
        JavaCmd::List => {
            let runtimes = client.java().list().await?;
            if runtimes.is_empty() {
                return ui::show(View::note("no java runtimes installed"));
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
                .collect();
            ui::show(View::table(
                "java runtimes",
                ["MAJOR", "VENDOR", "RELEASE", "HOME"],
                rows,
            ))?;
        }
        JavaCmd::Install { major, force } => {
            let reporter = Arc::new(InstallReporter::new());
            let progress = reporter.clone();
            let (runtime, already) = client
                .java()
                .install(major, force, move |p| progress.update(p))
                .await?;
            reporter.finish();
            let verb = if already {
                "already installed"
            } else {
                "installed"
            };
            ui::show(View::line(format!(
                "java {} {verb} ({})",
                runtime.major, runtime.release_name
            )))?;
        }
        JavaCmd::Uninstall { major } => {
            {
                let _spinner = Spinner::start(format!("uninstalling java {major}"));
                client.java().uninstall(major).await?;
            }
            ui::show(View::line(format!("uninstalled java {major}")))?;
        }
    }
    Ok(())
}
