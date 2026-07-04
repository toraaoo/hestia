//! The Tauri v2 desktop shell. The webview loads the self-contained root
//! `frontend/`; each `#[tauri::command]` is a thin proxy to the daemon over the
//! socket via the client SDK — the desktop never links the engine.

mod api;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![api::app_info, api::java_list])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
