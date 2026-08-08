//! The servers a key can see, and what it can read about them.
//!
//! Every route spends one line naming the scope it costs, and every route that
//! takes a `:id` resolves it through [`Api::server`] so a narrowed key cannot
//! reach past what it was issued for.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::Router;
use proto::content::{ContentKind, ServerContentList, ServerContentListParams};
use proto::remote::Scope;
use proto::server::{
    ServerConfigList, ServerDetail, ServerList, ServerLogs, ServerLogsParams, ServerPing,
    ServerRef, ServerStatus,
};
use serde::Deserialize;
use serde_json::json;

use crate::http::auth::Key;
use crate::http::envelope::{Answer, ApiResult};
use crate::http::Api;

pub(super) fn mount() -> Router<Api> {
    Router::new()
        .route("/servers", get(list))
        .route("/servers/{id}", get(status))
        .route("/servers/{id}/detail", get(detail))
        .route("/servers/{id}/ping", get(ping))
        .route("/servers/{id}/logs", get(logs))
        .route("/servers/{id}/config", get(config))
        .route("/servers/{id}/content", get(content))
}

async fn list(State(api): State<Api>, key: Key) -> ApiResult {
    let grant = key.require(Scope::ServerRead)?;
    let mut servers = api.call::<ServerList>(proto::Empty {}).await?.servers;
    // A narrowed key does not merely fail to open the other servers — it never
    // learns they exist.
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

/// Captured output. These lines are owner-only on disk for a reason — an old
/// client prints its own session id, and a server log names every player's
/// address — so this route costs a scope rather than riding along with `list`.
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
