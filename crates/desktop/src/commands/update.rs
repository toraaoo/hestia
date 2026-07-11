//! The in-app updater — the shell's own path to a new release.
//!
//! Deliberately not the generic bridge: `tauri-plugin-updater` runs the
//! platform installer and restarts the process, which only the shell can do.
//! The daemon's `update.*` channels cover the same ground for the CLI, and
//! both verify against the one minisign key in `tauri.conf.json`.

use serde::Serialize;
use tauri_plugin_updater::UpdaterExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
    notes: Option<String>,
}

/// Ask the release endpoint whether a newer version exists.
#[tauri::command]
pub async fn update_check(app: tauri::AppHandle) -> Result<Option<UpdateInfo>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    Ok(update.map(|u| UpdateInfo {
        version: u.version,
        notes: u.body,
    }))
}

/// Download and install the pending update, then restart into it. On Windows
/// the signed NSIS installer runs passively, reusing the recorded install
/// directory and component choices.
#[tauri::command]
pub async fn update_install(app: tauri::AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no update available")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart();
}
