//! Login-time autostart registration, driven by the reserved `autostart` config
//! key. Registers the running daemon's own executable so the registration
//! survives the binary being moved.

use anyhow::{bail, Context, Result};

pub const SUPPORTED: bool = !cfg!(debug_assertions);

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

pub fn set(enabled: bool) -> Result<()> {
    if enabled && !SUPPORTED {
        bail!("start at login is unavailable in debug builds");
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
