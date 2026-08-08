//! Discovery and liveness — the two paths that carry no key.
//!
//! Discovery is deliberately **unversioned**: a client that guessed the wrong
//! `/api/vN` has to be able to ask what this node serves without already knowing
//! ([0073](../../../../../docs/decisions/0073-the-http-surface-is-versioned-in-the-path.md)).
//! Neither route reveals anything a port scan does not already establish.

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
