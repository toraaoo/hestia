//! A server's archives. Creating and restoring one is a job: the call answers
//! 202 with its id and the outcome arrives on the event stream.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::Router;
use proto::backup::{
    ServerBackupCreate, ServerBackupCreateParams, ServerBackupList, ServerBackupRef,
    ServerBackupRemove, ServerBackupRestore, ServerBackupRestoreParams,
};
use proto::remote::Scope;
use proto::server::ServerRef;
use serde_json::json;

use crate::http::auth::Key;
use crate::http::envelope::{Answer, ApiResult};
use crate::http::{Api, CURRENT};

pub(super) fn mount() -> Router<Api> {
    Router::new()
        .route("/servers/{id}/backups", get(list).post(create))
        .route(
            "/servers/{id}/backups/{backup}",
            axum::routing::delete(remove),
        )
        .route("/servers/{id}/backups/{backup}/restore", post(restore))
}

async fn list(State(api): State<Api>, key: Key, Path(id): Path<String>) -> ApiResult {
    let server = api.server(key.require(Scope::ServerRead)?, &id)?;
    let listed = api.call::<ServerBackupList>(ServerRef { server }).await?;
    Ok(Answer::new(
        format!("{} backups", listed.backups.len()),
        json!({ "backups": listed.backups }),
    ))
}

async fn create(State(api): State<Api>, key: Key, Path(id): Path<String>) -> ApiResult {
    let server = api.server(key.require(Scope::ServerBackup)?, &id)?;
    let job = api
        .call::<ServerBackupCreate>(ServerBackupCreateParams {
            server,
            id: String::new(),
        })
        .await?;
    Ok(Answer::accepted("backup started", json!({ "id": job.id }))
        .at(format!("/api/{CURRENT}/events")))
}

async fn restore(
    State(api): State<Api>,
    key: Key,
    Path((id, backup)): Path<(String, String)>,
) -> ApiResult {
    let server = api.server(key.require(Scope::ServerBackup)?, &id)?;
    let job = api
        .call::<ServerBackupRestore>(ServerBackupRestoreParams {
            server,
            backup,
            id: String::new(),
        })
        .await?;
    Ok(Answer::accepted("restore started", json!({ "id": job.id }))
        .at(format!("/api/{CURRENT}/events")))
}

async fn remove(
    State(api): State<Api>,
    key: Key,
    Path((id, backup)): Path<(String, String)>,
) -> ApiResult {
    let server = api.server(key.require(Scope::ServerBackup)?, &id)?;
    api.call::<ServerBackupRemove>(ServerBackupRef { server, backup })
        .await?;
    Ok(Answer::new("backup removed", json!({})))
}
