mod bridge;

use bridge::Bridge;
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Bridge::default())
        .setup(|app| {
            bridge::watch(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![bridge::ipc_call])
        // The frontend keeps a hidden, pre-warmed dialog window; without this
        // the app would outlive its main window.
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, WindowEvent::Destroyed) {
                window.app_handle().exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
