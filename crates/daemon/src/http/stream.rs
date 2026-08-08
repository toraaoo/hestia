//! Server-sent events rather than WebSocket: the need is one-way, and console
//! input is an ordinary POST. The SSE `event:` field **is** the topic name and
//! `data:` **is** the topic payload verbatim, so there is no translation table
//! to keep in step with `proto`.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::stream::Stream;
use ipc::protocol::Event;
use proto::naming::server_process_id;
use proto::remote::Scope;
use tokio::sync::mpsc::UnboundedReceiver;

use super::auth::Key;
use super::envelope::Failure;
use super::{next_conn_id, Api, CURRENT};

/// Topics a `server:read` key may see. Everything else the daemon publishes
/// belongs to the machine's owner, not to a node operator — a stream is not a
/// way around the allowlist the routes enforce.
const SERVER_TOPICS: &[&str] = &[
    "server.create.progress",
    "server.create.done",
    "server.create.error",
    "server.create.cancelled",
    "server.update.progress",
    "server.update.done",
    "server.update.error",
    "server.update.cancelled",
    "backup.progress",
    "backup.done",
    "backup.error",
    "backup.cancelled",
    "content.progress",
    "content.done",
    "content.error",
    "content.cancelled",
    "modpack.progress",
    "modpack.done",
    "modpack.error",
    "modpack.cancelled",
    "process.started",
    "process.output",
    "process.exit",
    "process.metrics",
    "net.state",
];

fn is_server_topic(topic: &str) -> bool {
    SERVER_TOPICS.contains(&topic)
}

pub(super) fn mount() -> Router<Api> {
    Router::new()
        .route(&format!("/api/{CURRENT}/events"), get(events))
        .route(
            &format!("/api/{CURRENT}/servers/{{id}}/console"),
            get(console),
        )
}

/// Everything happening on the node that this key is entitled to see.
async fn events(State(api): State<Api>, key: Key) -> Result<impl IntoResponse, Failure> {
    let grant = key.require(Scope::ServerRead)?;
    Ok(open(&api, grant.id.clone(), None))
}

/// One server's captured output, live. The filter is the entry key, which covers
/// the process keys beneath it, so a console survives a restart.
async fn console(
    State(api): State<Api>,
    key: Key,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, Failure> {
    let grant = key.require(Scope::ServerRead)?;
    let key_id = grant.id.clone();
    let server = api.server(grant, &id)?;
    Ok(open(&api, key_id, Some(server_process_id(&server))))
}

fn open(
    api: &Api,
    key_id: String,
    filter: Option<String>,
) -> Sse<impl Stream<Item = Result<SseEvent, std::convert::Infallible>>> {
    let (out, rx) = tokio::sync::mpsc::unbounded_channel();
    api.runtime
        .hub()
        .subscribe_stream(next_conn_id(), out, filter, key_id);
    // Keep-alive comments; an intermediary proxy drops a connection that goes
    // quiet, and an idle server publishes nothing for hours.
    Sse::new(frames(rx)).keep_alive(KeepAlive::default())
}

fn frames(
    rx: UnboundedReceiver<Arc<Event>>,
) -> impl Stream<Item = Result<SseEvent, std::convert::Infallible>> {
    futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            let event = rx.recv().await?;
            if !is_server_topic(&event.topic) {
                continue;
            }
            let frame = SseEvent::default()
                .event(event.topic.clone())
                .json_data(&event.payload)
                .unwrap_or_default();
            return Some((Ok(frame), rx));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stream_carries_server_topics_and_no_others() {
        for allowed in ["backup.progress", "process.output", "server.create.done"] {
            assert!(is_server_topic(allowed), "{allowed} should reach a stream");
        }
        for withheld in [
            "instance.launch.progress",
            "instance.export.done",
            "instance.import.progress",
            "java.install.progress",
            "download.progress",
            "update.progress",
            "update.done",
        ] {
            assert!(
                !is_server_topic(withheld),
                "{withheld} is not a node operator's business"
            );
        }
    }

    /// The allowlist is spelled out, so a topic added to `proto` must be
    /// classified deliberately rather than inherited by prefix.
    #[test]
    fn the_topic_allowlist_holds_no_duplicates_and_no_instance_topics() {
        let mut sorted = SERVER_TOPICS.to_vec();
        sorted.sort_unstable();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(before, sorted.len(), "a topic is listed twice");
        assert!(SERVER_TOPICS.iter().all(|t| !t.starts_with("instance.")));
    }
}
