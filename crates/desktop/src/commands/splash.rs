//! The launch splash: a second window shown while the webview boots.
//!
//! The main window is created hidden, so nothing is on screen until the app
//! has mounted. The webview calls `ready` once it has; `arm` is the backstop
//! for the case where it never does, because a frontend that fails to boot
//! must still leave the user a window rather than an invisible process.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

const MIN_VISIBLE: Duration = Duration::from_millis(2500);

const FALLBACK: Duration = Duration::from_secs(10);

pub struct Splash {
    shown_at: Instant,
    claimed: AtomicBool,
}

impl Default for Splash {
    fn default() -> Self {
        Splash {
            shown_at: Instant::now(),
            claimed: AtomicBool::new(false),
        }
    }
}

/// Hide the splash and reveal the app, once the animation has had its time.
/// Idempotent: whichever of the webview and the backstop arrives first wins,
/// and the other becomes a no-op.
pub fn reveal(app: &AppHandle) {
    let Some(state) = app.try_state::<Splash>() else {
        return;
    };
    if state.claimed.swap(true, Ordering::SeqCst) {
        return;
    }
    let remaining = MIN_VISIBLE.saturating_sub(state.shown_at.elapsed());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(remaining).await;
        if let Some(splash) = app.get_webview_window("splashscreen") {
            let _ = splash.close();
        }
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.show();
            let _ = main.set_focus();
        }
    });
}

pub fn arm(app: AppHandle) {
    app.manage(Splash::default());
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FALLBACK).await;
        if !app.state::<Splash>().claimed.load(Ordering::SeqCst) {
            tracing::warn!("webview never reported ready; revealing anyway");
            reveal(&app);
        }
    });
}

#[tauri::command]
pub fn ready(app: AppHandle) {
    reveal(&app);
}
