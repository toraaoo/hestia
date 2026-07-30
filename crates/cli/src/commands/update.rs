//! `hestia self-update` — check the release feed and apply the new installer.

use std::sync::Arc;

use anyhow::{Context, Result};

use crate::ui::{self, DownloadReporter, Spinner, View};

pub async fn run(yes: bool) -> Result<()> {
    let client = super::connect().await?;
    let status = {
        let _spinner = Spinner::start("checking for updates");
        client.update().check().await?
    };
    let Some(info) = status.available else {
        return ui::show(View::line(format!(
            "hestia {} is up to date",
            status.current
        )));
    };

    if !apply::supported() {
        return ui::show(View::note(format!(
            "hestia {} is available (installed: {}) — download it at {}",
            info.version, status.current, info.url
        )));
    }

    if !yes {
        let accepted = ui::confirm(
            &format!("update hestia {} → {}?", status.current, info.version),
            "download and update",
            "cancel",
        )
        .context("pass --yes to update without a prompt")?;
        if !accepted {
            return ui::show(View::note("update cancelled"));
        }
    }

    let reporter = Arc::new(DownloadReporter::new("downloading update"));
    let progress = reporter.clone();
    let (path, version) = client
        .update()
        .download(move |p| progress.update(p))
        .await?;
    reporter.finish();

    apply::start(&path)?;
    ui::show(View::line(format!(
        "installer for {version} started — it stops the daemon, updates, and restarts it"
    )))
}

#[cfg(windows)]
mod apply {
    use std::path::Path;

    use anyhow::{Context, Result};

    /// Applying only makes sense under an NSIS-managed installation, marked
    /// by the uninstaller sitting beside this binary.
    pub fn supported() -> bool {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join("uninstall.exe").exists()))
            .unwrap_or(false)
    }

    /// Run the downloaded installer passively: /P shows only a progress page,
    /// /UPDATE reuses the recorded install dir, mode, and component choices.
    pub fn start(installer: &Path) -> Result<()> {
        std::process::Command::new(installer)
            .args(["/P", "/UPDATE"])
            .spawn()
            .context("cannot start the installer")?;
        Ok(())
    }
}

#[cfg(not(windows))]
mod apply {
    use std::path::Path;

    use anyhow::{bail, Result};

    pub fn supported() -> bool {
        false
    }

    pub fn start(_installer: &Path) -> Result<()> {
        bail!("self-update applies only to installer-managed installations");
    }
}
