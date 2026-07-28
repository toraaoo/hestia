//! Shared application identity — the single source of truth every front-end and
//! the daemon read, mirroring the generated `app_info.h` in the C++ tree.

pub const NAME: &str = "Hestia";
pub const ID: &str = "tech.lawrenceallen.hestia";
/// The tray's own GApplication/desktop id. It must differ from [`ID`] (the
/// desktop shell's Tauri identifier): both front-ends register a GApplication
/// under this name on Linux, and GApplication enforces single-instance by
/// D-Bus name ownership — sharing the id makes the second process launched a
/// remote instance that never shows, so the tray and desktop would block each
/// other.
pub const TRAY_ID: &str = "tech.lawrenceallen.hestia.tray";
/// The flag the tray passes to re-launch the desktop shell just to close it:
/// the running instance routes it through single-instance and exits.
pub const DESKTOP_QUIT_ARG: &str = "--quit";
pub const VENDOR: &str = "toraaoo";
pub const CHANNEL: &str = "dev";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The release manifest every front-end checks for a newer version, and the
/// minisign public key its artifacts are verified against. Both must match
/// `plugins.updater` in `crates/desktop/tauri.conf.json` — the desktop shell
/// reads them from there through `tauri-plugin-updater`, and
/// `crates/common/tests/updater.rs` fails the build when the two disagree.
pub const UPDATE_ENDPOINT: &str =
    "https://github.com/toraaoo/hestia/releases/latest/download/latest.json";
pub const UPDATE_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDJDNjM3NzcxQUEwRTdDQUQKUldTdGZBNnFjWGRqTERoaEIzaXFJcU1ZdU1YdXBVUk16cFdGVFQzYmZtT3ZVRC9mbjdYU0dOQlkK";
/// The agent every outbound HTTP request identifies itself with.
///
/// PaperMC and Modrinth both *ask* for a contact URL or address alongside the
/// name — so an upstream can reach a misbehaving client instead of blocking it
/// — but neither enforces it, and Hestia has no published home to name yet. A
/// contact belongs here when there is a real one; an invented URL would point
/// upstreams somewhere that is not us, which is worse than staying anonymous.
pub fn user_agent() -> String {
    format!("{NAME}/{VERSION}")
}

#[cfg(debug_assertions)]
pub const VERSION_LABEL: &str = concat!(env!("CARGO_PKG_VERSION"), "-debug");
#[cfg(not(debug_assertions))]
pub const VERSION_LABEL: &str = env!("CARGO_PKG_VERSION");
