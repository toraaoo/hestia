//! The protocol's error-code vocabulary in one place. The daemon raises these and
//! the client matches on them.

pub const NOT_FOUND: &str = "not_found";
pub const BAD_REQUEST: &str = "bad_request";
pub const HANDLER_ERROR: &str = "handler_error";
pub const UNKNOWN_CHANNEL: &str = "unknown_channel";
pub const VERSION_MISMATCH: &str = "version_mismatch";

use thiserror::Error;

/// A transport- or protocol-level failure surfaced to callers of the client SDK.
#[derive(Debug, Error)]
pub enum IpcError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("frame too large: {0} bytes")]
    FrameTooLarge(u32),
    #[error("malformed frame: {0}")]
    Malformed(String),
    #[error("daemon connection lost")]
    ConnectionLost,
    #[error("timed out waiting for daemon response on '{0}'")]
    Timeout(String),
    #[error("{code}: {message}")]
    Daemon { code: String, message: String },
}
