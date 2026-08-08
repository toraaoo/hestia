//! Discovery and liveness — the two paths that carry no key, revealing nothing
//! a port scan does not already establish. Discovery is unversioned so a client
//! that guessed the wrong `/api/vN` can ask
//! ([0073](../../../../../docs/decisions/0073-the-http-surface-is-versioned-in-the-path.md)).

use axum::routing::get;
use axum::Router;
use ipc::PROTOCOL_VERSION;
use serde_json::json;

use crate::http::envelope::{Answer, ApiResult};
use crate::http::{Api, VERSIONS};

pub(super) fn mount() -> Router<Api> {
    Router::new()
        .route("/api/versions", get(versions))
        .route("/health", get(health))
}

async fn versions() -> ApiResult {
    Ok(Answer::new(
        "hestiad",
        json!({
            "versions": VERSIONS,
            "daemon": common::app::VERSION,
            "protocol": PROTOCOL_VERSION,
        }),
    ))
}

async fn health() -> ApiResult {
    Ok(Answer::new("alive", json!({ "status": "alive" })))
}
