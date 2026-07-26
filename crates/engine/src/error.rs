//! The canonical `anyhow` → `ErrorInfo` projection, shared by the daemon's
//! request handlers and its job managers so a failure localizes identically
//! whether it returns from a request or arrives as a job's error event.

use proto::error::ErrorInfo;

use crate::ReauthRequired;

/// Project an engine `anyhow` failure onto the wire `ErrorInfo`: a typed
/// `ErrorInfo` the engine raised passes through untouched, a re-auth need
/// becomes `SessionExpired`, and anything else is an opaque `Internal` carrying
/// its English chain as `detail`.
pub fn error_info(error: anyhow::Error) -> ErrorInfo {
    match error.downcast::<ErrorInfo>() {
        Ok(info) => info,
        Err(error) => {
            if let Some(reauth) = error.downcast_ref::<ReauthRequired>() {
                return ErrorInfo::SessionExpired {
                    reference: reauth.reference.clone(),
                };
            }
            tracing::error!(error = ?error, "unhandled engine failure");
            ErrorInfo::Internal {
                detail: format!("{error:#}"),
            }
        }
    }
}
