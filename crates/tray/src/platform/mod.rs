//! The status area, per platform. Each backend owns how an icon is published
//! and how a menu is rendered; everything above it — the daemon polling, the
//! menu's vocabulary, the icon raster — is shared.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::run;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::run;
