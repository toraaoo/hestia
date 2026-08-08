//! The allowlist: every path the door answers, one file per resource. An
//! unnamed channel is not refused — it has no route at all
//! ([0075](../../../../../docs/decisions/0075-the-remote-surface-is-an-allowlist.md)).
//! `meta` is the unversioned discovery pair; the rest mount under `/api/v1`.

mod backup;
mod meta;
mod server;

use axum::http::StatusCode;
use axum::Router;
use tower_http::timeout::TimeoutLayer;

use super::{Api, CURRENT, REQUEST_TIMEOUT};

pub(super) fn mount() -> Router<Api> {
    Router::new()
        .merge(meta::mount())
        .nest(
            &format!("/api/{CURRENT}"),
            server::mount().merge(backup::mount()),
        )
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            REQUEST_TIMEOUT,
        ))
}
