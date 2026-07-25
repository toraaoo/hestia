#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKitGTK's DMABUF renderer leaves the webview blank under some Wayland
    // compositors (Hyprland) and on Nvidia; disable it before startup.
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    let level = common::LogLevel::default();
    let file = common::FileLog::for_binary("desktop", None, level).with_firehose();
    let _log_guard = common::init_logging(common::LogLevel::Warn, Some(file));
    desktop::run();
}
