mod bridge;

use bridge::Bridge;

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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
