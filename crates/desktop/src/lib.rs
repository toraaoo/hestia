mod bridge;
mod commands;

use bridge::Bridge;
use tauri::Manager;

fn is_quit_signal(argv: &[String]) -> bool {
    argv.iter().any(|arg| arg == common::app::DESKTOP_QUIT_ARG)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Single-instance must be registered first: a second launch (e.g. the
        // tray's left-click) hands its args to the running instance and exits,
        // and we surface the existing window instead of opening another.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if is_quit_signal(&argv) {
                app.exit(0);
                return;
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Bridge::default())
        .setup(|app| {
            if is_quit_signal(&std::env::args().collect::<Vec<_>>()) {
                app.handle().exit(0);
                return Ok(());
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }
            bridge::watch(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bridge::ipc_call,
            commands::auth::account_login_sisu,
            commands::prefs::prefs_list,
            commands::prefs::prefs_set,
            commands::prefs::prefs_remove,
            commands::icons::icons_list,
            commands::icons::icon_set,
            commands::icons::icon_remove
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
