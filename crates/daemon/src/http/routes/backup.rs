//! A server's archives. Read-only in phase 1 — creating, restoring and deleting
//! one costs `server:backup`, which nothing mints yet.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::Router;
use proto::backup::ServerBackupList;
use proto::remote::Scope;
use proto::server::ServerRef;
use serde_json::json;

use crate::http::auth::Key;
use crate::http::envelope::{Answer, ApiResult};
use crate::http::Api;

pub(super) fn mount() -> Router<Api> {
    Router::new().route("/servers/{id}/backups", get(list))
}

async fn list(State(api): State<Api>, key: Key, Path(id): Path<String>) -> ApiResult {
    let server = api.server(key.require(Scope::ServerRead)?, &id)?;
    let listed = api.call::<ServerBackupList>(ServerRef { server }).await?;
    Ok(Answer::new(
        format!("{} backups", listed.backups.len()),
        json!({ "backups": listed.backups }),
    ))
}
