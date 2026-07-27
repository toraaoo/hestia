//! The protocol's error-code vocabulary in one place. The daemon raises these and
//! the client matches on them.

pub const NOT_FOUND: &str = "not_found";
pub const BAD_REQUEST: &str = "bad_request";
pub const HANDLER_ERROR: &str = "handler_error";
pub const UNKNOWN_CHANNEL: &str = "unknown_channel";
pub const VERSION_MISMATCH: &str = "version_mismatch";
pub const UNAUTHORIZED: &str = "unauthorized";

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
    #[error(
        "incompatible protocol version: the daemon speaks version {got}, this build speaks {want}"
    )]
    IncompatibleVersion { got: i64, want: i64 },
    #[error("timed out waiting for daemon response on '{0}'")]
    Timeout(String),
    /// The job was cancelled at someone's request. Not a failure — nothing went
    /// wrong — so a front-end reports it as such rather than as an error.
    #[error("cancelled")]
    Cancelled,
    #[error("{message}")]
    Daemon {
        code: String,
        message: String,
        /// The raw serialized `proto::error::ErrorInfo` for structured consumers
        /// (the desktop forwards it to the webview to localize); `code` and
        /// `message` are the client's local projection of it.
        info: serde_json::Value,
    },
}
