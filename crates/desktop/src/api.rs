//! Tauri commands. Each connects to the daemon through the client SDK and returns
//! `proto` types directly; errors become strings the webview can surface.

use client::proto::app::AppInfoResult;
use client::proto::java::JavaRuntime;

async fn connect() -> Result<client::Client, String> {
    client::Client::connect(true).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn app_info() -> Result<AppInfoResult, String> {
    connect().await?.app().info().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn java_list() -> Result<Vec<JavaRuntime>, String> {
    connect().await?.java().list().await.map_err(|e| e.to_string())
}
