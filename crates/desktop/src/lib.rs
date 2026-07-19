mod bridge;
mod commands;

use bridge::Bridge;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Bridge::default())
        .setup(|app| {
            bridge::watch(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bridge::ipc_call,
            commands::auth::account_login_sisu,
            commands::prefs::prefs_list,
            commands::prefs::prefs_set,
            commands::prefs::prefs_remove
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
