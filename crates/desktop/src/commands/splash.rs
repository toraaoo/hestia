//! The launch splash: a second window shown while the webview boots.
//!
//! The main window is created hidden, so nothing is on screen until the app
//! has actually painted. The webview calls `ready` once mounted; `arm` is the
//! backstop for the case where it never does, because a frontend that fails to
//! boot must still leave the user a window rather than an invisible process.

use std::time::Duration;

use tauri::{AppHandle, Manager};

const FALLBACK: Duration = Duration::from_secs(10);

/// Hide the splash and reveal the app. Idempotent: whichever of the webview
/// and the backstop arrives first wins, and the other becomes a no-op.
pub fn reveal(app: &AppHandle) {
    if let Some(splash) = app.get_webview_window("splashscreen") {
        let _ = splash.close();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
}

pub fn arm(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FALLBACK).await;
        if app
            .get_webview_window("splashscreen")
            .is_some_and(|w| w.is_visible().unwrap_or(false))
        {
            tracing::warn!("webview never reported ready; revealing anyway");
            reveal(&app);
        }
    });
}

#[tauri::command]
pub fn ready(app: AppHandle) {
    reveal(&app);
}
