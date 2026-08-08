//! The allowlist. Every path the door answers is here, and a channel that is not
//! named is not merely refused — it has no route, so it 404s on an unmounted
//! path. Adding a channel to `services/` therefore never widens this surface
//! ([0075](../../../../../docs/decisions/0075-the-remote-surface-is-an-allowlist.md)).
//!
//! Grouped one file per resource. `meta` is the unversioned discovery pair; the
//! rest mount under `/api/v1`.

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
