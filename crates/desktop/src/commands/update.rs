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
///
/// The plugin's config holds one `pubkey` and verifies against only that, so
/// each trusted key gets its own builder-overridden updater: after a signing
/// key rotation the artifact is signed by the successor, which a build in the
/// field knows but does not have configured. Retrying re-downloads, which is
/// acceptable for something that happens once per rotation.
#[tauri::command]
pub async fn update_install(app: tauri::AppHandle) -> Result<(), String> {
    let mut last_error = None;
    for pubkey in common::app::update_pubkeys() {
        let updater = app
            .updater_builder()
            .pubkey(pubkey)
            .build()
            .map_err(|e| e.to_string())?;
        let update = updater
            .check()
            .await
            .map_err(|e| e.to_string())?
            .ok_or("no update available")?;
        match update.download_and_install(|_, _| {}, || {}).await {
            Ok(()) => app.restart(),
            Err(e) => last_error = Some(e.to_string()),
        }
    }
    Err(last_error.unwrap_or_else(|| "no trusted key verifies this update".to_string()))
}
