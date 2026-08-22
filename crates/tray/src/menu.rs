//! The tray menu's vocabulary: what each entry reads as for a given daemon
//! state. The entries themselves are built per platform — Linux rebuilds the
//! menu from state on every render, Windows mutates persistent widgets — so
//! only the text and the enablement rules live here, where both read them.

use crate::worker::DaemonState;

/// Autostart is a login-item registration, which a debug build must not make.
pub const AUTOSTART_SUPPORTED: bool = !cfg!(debug_assertions);

pub const AUTOSTART_LABEL: &str = "Start at login";

pub fn open_label() -> String {
    format!("Open {}", common::app::NAME)
}

pub fn quit_label() -> String {
    format!("Quit {}", common::app::NAME)
}

pub fn daemon_label(state: &DaemonState) -> &'static str {
    if state.running {
        "Restart daemon"
    } else {
        "Start daemon"
    }
}

pub fn autostart_enabled(state: &DaemonState) -> bool {
    state.running && AUTOSTART_SUPPORTED
}

pub fn status_text(state: &DaemonState) -> String {
    match &state.version {
        Some(version) => format!("{} {version} — running", common::app::NAME),
        None => format!("{} — stopped", common::app::NAME),
    }
}

pub fn tooltip(state: &DaemonState) -> String {
    let condition = if state.running { "running" } else { "stopped" };
    format!("{} — {condition}", common::app::NAME)
}
