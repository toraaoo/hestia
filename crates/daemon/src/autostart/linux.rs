use std::path::PathBuf;

use anyhow::{Context, Result};

const ENTRY_NAME: &str = "hestiad.desktop";

fn autostart_dir() -> Result<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Ok(PathBuf::from(xdg).join("autostart"));
        }
    }
    let home = std::env::var_os("HOME").context("cannot resolve autostart dir: HOME is unset")?;
    Ok(PathBuf::from(home).join(".config").join("autostart"))
}

fn entry_contents() -> Result<String> {
    let exe = std::env::current_exe().context("cannot resolve daemon executable path")?;
    Ok(include_str!("../../assets/autostart.desktop")
        .replace("@NAME@", common::app::NAME)
        .replace("@EXEC@", &exe.display().to_string()))
}

pub(super) fn enable() -> Result<()> {
    let dir = autostart_dir()?;
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(ENTRY_NAME), entry_contents()?).context("cannot enable autostart")
}

pub(super) fn disable() -> Result<()> {
    let dir = autostart_dir()?;
    let _ = std::fs::remove_file(dir.join(ENTRY_NAME));
    Ok(())
}

pub(super) fn is_enabled() -> bool {
    autostart_dir()
        .map(|dir| dir.join(ENTRY_NAME).exists())
        .unwrap_or(false)
}
