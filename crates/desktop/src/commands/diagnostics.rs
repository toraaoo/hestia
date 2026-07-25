use std::path::PathBuf;

use crate::bridge::CallError;

const TARGET: &str = "ui";

#[tauri::command]
pub fn log_write(level: String, message: String, fields: Option<String>) {
    let fields = fields.unwrap_or_default();
    match level.as_str() {
        "trace" => tracing::trace!(target: TARGET, %fields, "{message}"),
        "debug" => tracing::debug!(target: TARGET, %fields, "{message}"),
        "warn" => tracing::warn!(target: TARGET, %fields, "{message}"),
        "error" | "fatal" => tracing::error!(target: TARGET, %fields, "{message}"),
        _ => tracing::info!(target: TARGET, %fields, "{message}"),
    }
}

#[tauri::command]
pub fn crash_report(
    kind: String,
    message: String,
    location: String,
    detail: String,
) -> Option<String> {
    common::crash::record(&kind, &message, &location, &detail)
        .map(|path| path.display().to_string())
}

#[tauri::command]
pub fn crash_list() -> Vec<String> {
    common::crash::list()
        .into_iter()
        .map(|path| path.display().to_string())
        .collect()
}

#[tauri::command]
pub fn crash_read(path: String) -> Result<String, CallError> {
    common::crash::read(&PathBuf::from(path)).map_err(|e| CallError::other(e.to_string()))
}

#[tauri::command]
pub fn crash_clear() -> Result<(), CallError> {
    common::crash::clear().map_err(|e| CallError::other(e.to_string()))
}
