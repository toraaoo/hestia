//! Login-time autostart registration, driven by the reserved `autostart` config
//! key. Registers the running daemon's own executable so the registration
//! survives the binary being moved.

use anyhow::{bail, Context, Result};

/// A portable build is deliberately excluded: registering autostart would write
/// an absolute path into the login session pointing at a directory the user can
/// move, rename, or unplug, leaving a broken entry behind on the host — the one
/// trace a portable install exists to avoid.
pub const SUPPORTED: bool = !cfg!(debug_assertions) && !cfg!(feature = "portable");

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as backend;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as backend;

#[cfg(not(any(target_os = "linux", windows)))]
mod unsupported;
#[cfg(not(any(target_os = "linux", windows)))]
use unsupported as backend;

pub fn is_enabled() -> bool {
    SUPPORTED && backend::is_enabled()
}

/// Why [`SUPPORTED`] is false, so a client is told which build it is talking to
/// rather than a reason that does not apply.
const UNSUPPORTED_REASON: &str = if cfg!(feature = "portable") {
    "start at login is unavailable in a portable build"
} else {
    "start at login is unavailable in debug builds"
};

pub fn set(enabled: bool) -> Result<()> {
    if enabled && !SUPPORTED {
        bail!(UNSUPPORTED_REASON);
    }
    let result = if enabled {
        backend::enable().context("failed to enable autostart")
    } else {
        backend::disable().context("failed to disable autostart")
    };
    if result.is_ok() {
        tracing::info!(enabled, "autostart registration changed");
    }
    result
}
