//! The shell's bespoke Tauri commands — the deliberate exceptions to the
//! generic `bridge::ipc_call` pipe, one module per feature.

pub mod auth;
pub mod prefs;
