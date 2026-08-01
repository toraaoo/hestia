//! Shared application identity — the single source of truth every front-end and
//! the daemon read, mirroring the generated `app_info.h` in the C++ tree.

pub const NAME: &str = "Hestia";
pub const ID: &str = "org.prytaneum.hestia";
/// The tray's own GApplication/desktop id. It must differ from [`ID`] (the
/// desktop shell's Tauri identifier): both front-ends register a GApplication
/// under this name on Linux, and GApplication enforces single-instance by
/// D-Bus name ownership — sharing the id makes the second process launched a
/// remote instance that never shows, so the tray and desktop would block each
/// other.
pub const TRAY_ID: &str = "org.prytaneum.hestia.tray";
/// The flag the tray passes to re-launch the desktop shell just to close it:
/// the running instance routes it through single-instance and exits.
pub const DESKTOP_QUIT_ARG: &str = "--quit";
pub const VENDOR: &str = "prytaneum";
pub const CHANNEL: &str = "dev";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Shipped name first, cargo's output last, so a dev build still resolves.
#[cfg(windows)]
pub const DESKTOP_BIN: &[&str] = &["Hestia.exe", "hestia-desktop.exe"];
#[cfg(not(windows))]
pub const DESKTOP_BIN: &[&str] = &["hestia-desktop"];

#[cfg(windows)]
pub const TRAY_BIN: &[&str] = &["Hestia Tray.exe", "hestia-tray.exe"];
#[cfg(not(windows))]
pub const TRAY_BIN: &[&str] = &["hestia-tray"];

#[cfg(windows)]
pub const DAEMON_BIN: &[&str] = &["hestiad.exe"];
#[cfg(not(windows))]
pub const DAEMON_BIN: &[&str] = &["hestiad"];

/// The release manifest the daemon checks for a newer version, and the minisign
/// public key its artifacts are verified against. Every front-end reaches this
/// through `update.check`, so these two constants are the only place either is
/// written down.
pub const UPDATE_ENDPOINT: &str =
    "https://github.com/toraaoo/hestia/releases/latest/download/latest.json";
pub const UPDATE_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDJDNjM3NzcxQUEwRTdDQUQKUldTdGZBNnFjWGRqTERoaEIzaXFJcU1ZdU1YdXBVUk16cFdGVFQzYmZtT3ZVRC9mbjdYU0dOQlkK";

/// The rotation spare. A binary trusts only the keys compiled into it, so a
/// successor must ship *before* it is ever needed — an empty slot here cannot
/// be filled in retrospectively for copies already installed. Generate it in
/// the same session as [`UPDATE_PUBKEY`], keep its private half offline and
/// apart from the signing key, and start signing with it only once the builds
/// that trust it are the ones in the field.
pub const UPDATE_PUBKEY_NEXT: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IENEQzQxRUVGQkU3M0UyRjkKUldUNTRuTys3eDdFelFCQXM2R21VbTN2RjljYTRpSGZOOURCclhxdGJuR0JNZlpwWEJCOHVaazQK";

/// Every key a release artifact may be signed with, newest last. Empty slots
/// are skipped, so an unrotated build simply trusts one key.
pub fn update_pubkeys() -> impl Iterator<Item = &'static str> {
    [UPDATE_PUBKEY, UPDATE_PUBKEY_NEXT]
        .into_iter()
        .filter(|key| !key.is_empty())
}

/// The announcement feed: the news and notices the launcher shows. A standing
/// `announcements` release tag whose asset is replaced in place, so publishing
/// is decoupled from cutting a version — `releases/latest/` would tie the two
/// together and 404 on any release that omitted the asset.
pub const ANNOUNCE_ENDPOINT: &str =
    "https://github.com/toraaoo/hestia/releases/download/announcements/announcements.json";

/// The Discord application Rich Presence is published as, from the Discord
/// developer portal. Not a secret and not a credential: it names the
/// application whose title and art the Discord client renders, and it travels
/// in every presence payload. There is no backend behind it — presence is local
/// IPC to the user's own Discord client, so the application exists only to hold
/// the name and the uploaded art assets.
pub const DISCORD_APP_ID: &str = "1532750283753656543";

/// The art asset key uploaded under the application's Rich Presence assets.
/// Discord resolves it against that application; an unknown key renders no
/// image rather than failing the update.
pub const DISCORD_LARGE_IMAGE: &str = "hestia";

/// The announcement feed's own signing key — deliberately *not* [`UPDATE_PUBKEY`].
///
/// The feed is published by a workflow that runs on a push to the default
/// branch, while installers are signed only from a release tag. Sharing one key
/// would put the installer-signing secret within reach of anything that can land
/// a commit, so the lower-stakes artifact gets its own trust root: a compromised
/// announcement key can say things, never ship code.
///
/// Generated with `cargo tauri signer generate` (which writes the public half
/// already base64-wrapped, so it is pasted here verbatim). **An empty key set
/// fails closed** — the engine refuses an unverifiable feed rather than
/// trusting it — so announcements do not appear until this is filled in.
pub const ANNOUNCE_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDM5NTNFNDNFMDM2ODJBOEMKUldTTUttZ0RQdVJUT1RhdTBLTU9UMW4rbnkvQnpRdzN1K1JiNGhTVUxFWGZFdjFUeSs2bUI2UTQK";

/// The announcement rotation spare, with the same rules as [`UPDATE_PUBKEY_NEXT`]:
/// a binary trusts only what is compiled into it, so the successor must ship
/// before it is needed.
pub const ANNOUNCE_PUBKEY_NEXT: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDg2QzZFMjFENTFDOUQ5MDYKUldRRzJjbFJIZUxHaGxaRXgzTjlTd0N4SDlaazN6QWhTWHJrZGhoby8wMTRQY05ZYisyaDlRYkMK";

/// Every key the announcement feed may be signed with, newest last.
pub fn announce_pubkeys() -> impl Iterator<Item = &'static str> {
    [ANNOUNCE_PUBKEY, ANNOUNCE_PUBKEY_NEXT]
        .into_iter()
        .filter(|key| !key.is_empty())
}
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
