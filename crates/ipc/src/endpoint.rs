//! Endpoint (socket/pipe path) resolution for the daemon transport. A transport
//! concern, not a data concern: the runtime dir holds the ephemeral socket and is
//! distinct from the engine's persistent data_home.

use std::path::PathBuf;

/// The per-user runtime directory for ephemeral transport state.
#[cfg(unix)]
pub fn runtime_dir() -> PathBuf {
    // Prefer the session runtime dir (tmpfs, auto-cleaned at logout); fall back to
    // a uid-scoped /tmp dir so two users never collide on one socket path.
    if let Some(xdg) = std::env::var_os("XDG_RUNTIME_DIR") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("hestia");
        }
    }
    // SAFETY: getuid is always successful and has no preconditions.
    let uid = unsafe { libc::getuid() };
    PathBuf::from("/tmp").join(format!("hestia-{uid}"))
}

#[cfg(windows)]
pub fn runtime_dir() -> PathBuf {
    std::env::temp_dir().join("hestia")
}

/// The default daemon endpoint. Both the daemon (bind) and clients (connect)
/// resolve the same path here.
#[cfg(unix)]
pub fn default_endpoint() -> PathBuf {
    runtime_dir().join("hestiad.sock")
}

#[cfg(windows)]
pub fn default_endpoint() -> PathBuf {
    PathBuf::from(r"\\.\pipe\hestia-hestiad")
}
