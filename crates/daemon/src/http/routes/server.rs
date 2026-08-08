//! The servers a key can see and what it may do to them. Every route names the
//! scope it costs, and every `{id}` resolves through `Api::server` so a narrowed
//! key cannot reach past what it was issued for.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use proto::content::{ContentKind, ServerContentList, ServerContentListParams};
use proto::remote::Scope;
use proto::server::{
    ServerCommand, ServerCommandParams, ServerConfigList, ServerConfigSet, ServerConfigSetParams,
    ServerDetail, ServerList, ServerLogs, ServerLogsParams, ServerPing, ServerRef, ServerRename,
    ServerRenameParams, ServerRestart, ServerStart, ServerStatus, ServerStop,
};
use serde::Deserialize;
use serde_json::json;

use crate::http::auth::Key;
use crate::http::envelope::{Answer, ApiResult};
use crate::http::Api;

pub(super) fn mount() -> Router<Api> {
    Router::new()
        .route("/servers", get(list))
        .route("/servers/{id}", get(status).patch(rename))
        .route("/servers/{id}/detail", get(detail))
        .route("/servers/{id}/ping", get(ping))
        .route("/servers/{id}/logs", get(logs))
        .route("/servers/{id}/config", get(config))
        .route("/servers/{id}/config/{key}", put(configure))
        .route("/servers/{id}/content", get(content))
        .route("/servers/{id}/start", post(start))
        .route("/servers/{id}/stop", post(stop))
        .route("/servers/{id}/restart", post(restart))
        .route("/servers/{id}/console", post(command))
}

async fn list(State(api): State<Api>, key: Key) -> ApiResult {
    let grant = key.require(Scope::ServerRead)?;
    let mut servers = api.call::<ServerList>(proto::Empty {}).await?.servers;
    // A narrowed key must not learn the other servers exist.
    servers.retain(|server| grant.covers(&server.id));
    Ok(Answer::new(
        format!("{} servers", servers.len()),
        json!({ "servers": servers }),
    ))
}

async fn status(State(api): State<Api>, key: Key, Path(id): Path<String>) -> ApiResult {
    let server = api.server(key.require(Scope::ServerRead)?, &id)?;
    let info = api.call::<ServerStatus>(ServerRef { server }).await?;
    Ok(Answer::new(info.name.clone(), json!(info)))
}

async fn detail(State(api): State<Api>, key: Key, Path(id): Path<String>) -> ApiResult {
    let server = api.server(key.require(Scope::ServerRead)?, &id)?;
    let details = api.call::<ServerDetail>(ServerRef { server }).await?;
    Ok(Answer::new("server details", json!(details)))
}

/// A Server List Ping over the game port: what a player's multiplayer list would
/// show. Only a running server answers.
async fn ping(State(api): State<Api>, key: Key, Path(id): Path<String>) -> ApiResult {
    let server = api.server(key.require(Scope::ServerRead)?, &id)?;
    let ping = api.call::<ServerPing>(ServerRef { server }).await?;
    Ok(Answer::new("server ping", json!(ping)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Tail {
    tail: Option<usize>,
}

/// Captured output. Owner-only on disk because a server log names every
/// player's address, so this route costs a scope rather than riding on `list`.
async fn logs(
    State(api): State<Api>,
    key: Key,
    Path(id): Path<String>,
    Query(tail): Query<Tail>,
) -> ApiResult {
    let server = api.server(key.require(Scope::ServerRead)?, &id)?;
    let logs = api
        .call::<ServerLogs>(ServerLogsParams {
            server,
            tail: tail.tail,
        })
        .await?;
    Ok(Answer::new(
        format!("{} lines", logs.lines.len()),
        json!({ "lines": logs.lines }),
    ))
}

async fn config(State(api): State<Api>, key: Key, Path(id): Path<String>) -> ApiResult {
    let server = api.server(key.require(Scope::ServerRead)?, &id)?;
    let config = api.call::<ServerConfigList>(ServerRef { server }).await?;
    Ok(Answer::new(
        format!("{} settings", config.entries.len()),
        json!({ "entries": config.entries }),
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Kind {
    #[serde(default)]
    kind: ContentKind,
}

async fn content(
    State(api): State<Api>,
    key: Key,
    Path(id): Path<String>,
    Query(kind): Query<Kind>,
) -> ApiResult {
    let server = api.server(key.require(Scope::ServerRead)?, &id)?;
    let listed = api
        .call::<ServerContentList>(ServerContentListParams {
            server,
            kind: kind.kind,
        })
        .await?;
    Ok(Answer::new(
        format!("{} installed", listed.items.len()),
        json!(listed),
    ))
}

async fn start(State(api): State<Api>, key: Key, Path(id): Path<String>) -> ApiResult {
    let server = api.server(key.require(Scope::ServerControl)?, &id)?;
    let started = api.call::<ServerStart>(ServerRef { server }).await?;
    Ok(Answer::new("server started", json!(started)))
}

async fn stop(State(api): State<Api>, key: Key, Path(id): Path<String>) -> ApiResult {
    let server = api.server(key.require(Scope::ServerControl)?, &id)?;
    api.call::<ServerStop>(ServerRef { server }).await?;
    Ok(Answer::new("server stopping", json!({})))
}

async fn restart(State(api): State<Api>, key: Key, Path(id): Path<String>) -> ApiResult {
    let server = api.server(key.require(Scope::ServerControl)?, &id)?;
    let started = api.call::<ServerRestart>(ServerRef { server }).await?;
    Ok(Answer::new("server restarted", json!(started)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Command {
    command: String,
}

/// Console input — the one thing a client sends *to* a server, and why the
/// stream can stay one-way.
async fn command(
    State(api): State<Api>,
    key: Key,
    Path(id): Path<String>,
    Json(body): Json<Command>,
) -> ApiResult {
    let server = api.server(key.require(Scope::ServerControl)?, &id)?;
    let answer = api
        .call::<ServerCommand>(ServerCommandParams {
            server,
            command: body.command,
        })
        .await?;
    Ok(Answer::new(
        "command sent",
        json!({ "response": answer.response }),
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Rename {
    name: String,
}

async fn rename(
    State(api): State<Api>,
    key: Key,
    Path(id): Path<String>,
    Json(body): Json<Rename>,
) -> ApiResult {
    let server = api.server(key.require(Scope::ServerWrite)?, &id)?;
    let renamed = api
        .call::<ServerRename>(ServerRenameParams {
            server,
            name: body.name,
        })
        .await?;
    Ok(Answer::new(
        format!("renamed to {}", renamed.name),
        json!(renamed),
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Setting {
    value: String,
}

/// One setting per request: `server.config.set` writes one key, so a batch route
/// would be a multi-step operation this layer owned and could half-apply.
async fn configure(
    State(api): State<Api>,
    auth: Key,
    Path((id, key)): Path<(String, String)>,
    Json(body): Json<Setting>,
) -> ApiResult {
    let server = api.server(auth.require(Scope::ServerWrite)?, &id)?;
    api.call::<ServerConfigSet>(ServerConfigSetParams {
        server,
        key: key.clone(),
        value: body.value,
    })
    .await?;
    Ok(Answer::new(format!("{key} applied"), json!({ "key": key })))
}
