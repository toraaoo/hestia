//! The daemon bridge: one shared client connection held as Tauri state, a
//! generic `ipc_call` command that forwards the frontend's typed calls over
//! the socket, and daemon-event forwarding into the webview.
//!
//! The shell deliberately does not mirror the client facades as one Tauri
//! command per channel: the daemon already validates every payload through
//! the wire contract (`bad_request` / `unknown_channel`), so a per-channel
//! Rust layer would only add a third naming seam that can drift. The typed
//! surface lives once, in the frontend's `src/api/`.

use std::sync::Arc;
use std::time::Duration;

use client::proto::events::{EventsSubscribe, EventsSubscribeParams};
use client::{Client, IpcError};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

/// The webview event carrying every daemon push: `{ topic, payload }`.
pub const EVENT_CHANNEL: &str = "hestia:event";
/// The webview event carrying connection-state transitions.
pub const CONNECTION_CHANNEL: &str = "hestia:connection";

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const WATCH_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct Bridge {
    client: Mutex<Option<Arc<Client>>>,
}

#[derive(Serialize, Clone)]
struct EventPayload {
    topic: String,
    payload: Value,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ConnectionState {
    Connected,
    Disconnected,
}

/// The uniform rejection shape of `ipc_call`: daemon error codes pass through
/// (`not_found`, `bad_request`, …); transport failures get stable codes of
/// their own (`timeout`, `connection_lost`, `transport`).
#[derive(Serialize, Debug)]
pub struct CallError {
    code: String,
    message: String,
    /// The serialized `proto::error::ErrorInfo` the webview localizes from;
    /// `null` for transport failures with no daemon error.
    info: Value,
}

impl CallError {
    /// A shell-side failure with no daemon error code of its own.
    pub(crate) fn other(message: impl Into<String>) -> Self {
        CallError {
            code: "error".into(),
            message: message.into(),
            info: Value::Null,
        }
    }
}

impl From<IpcError> for CallError {
    fn from(error: IpcError) -> Self {
        let (code, info) = match &error {
            IpcError::Daemon { code, info, .. } => (code.clone(), info.clone()),
            IpcError::Timeout(_) => ("timeout".into(), Value::Null),
            IpcError::ConnectionLost => ("connection_lost".into(), Value::Null),
            _ => ("transport".into(), Value::Null),
        };
        CallError {
            code,
            message: error.to_string(),
            info,
        }
    }
}

#[tauri::command]
pub async fn ipc_call(
    app: AppHandle,
    bridge: State<'_, Bridge>,
    channel: String,
    payload: Value,
    timeout_ms: Option<u64>,
) -> Result<Value, CallError> {
    let client = acquire(&app, &bridge).await?;
    let timeout = timeout_ms.map_or(DEFAULT_TIMEOUT, Duration::from_millis);
    match client.session().call_raw(&channel, payload, timeout).await {
        Ok(response) if response.ok => Ok(response.payload),
        Ok(response) => {
            let raw = response.error.unwrap_or(Value::Null);
            let info = serde_json::from_value::<client::proto::error::ErrorInfo>(raw.clone())
                .unwrap_or(client::proto::error::ErrorInfo::Internal {
                    detail: "daemon error".into(),
                });
            Err(CallError {
                code: info.code().into(),
                message: info.to_string(),
                info: raw,
            })
        }
        Err(error) => {
            if client.session().is_closed() {
                release(&app, &bridge, &client).await;
            }
            Err(error.into())
        }
    }
}

/// Watch the shared connection: notice a lost daemon between calls and
/// passively reconnect (no auto-spawn — a deliberately stopped daemon must
/// stay stopped) so event forwarding resumes when it comes back.
pub fn watch(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(WATCH_INTERVAL).await;
            let bridge = app.state::<Bridge>();
            let mut guard = bridge.client.lock().await;
            match guard.as_ref() {
                Some(client) if client.session().is_closed() => {
                    *guard = None;
                    let _ = app.emit(CONNECTION_CHANNEL, ConnectionState::Disconnected);
                }
                Some(_) => {}
                None => {
                    if let Ok(client) = connect(&app, false).await {
                        *guard = Some(client);
                    }
                }
            }
        }
    });
}

pub(crate) async fn acquire(app: &AppHandle, bridge: &Bridge) -> Result<Arc<Client>, CallError> {
    let mut guard = bridge.client.lock().await;
    if let Some(client) = guard.as_ref() {
        if !client.session().is_closed() {
            return Ok(client.clone());
        }
        *guard = None;
        let _ = app.emit(CONNECTION_CHANNEL, ConnectionState::Disconnected);
    }
    let client = connect(app, true).await?;
    *guard = Some(client.clone());
    Ok(client)
}

async fn release(app: &AppHandle, bridge: &Bridge, lost: &Arc<Client>) {
    let mut guard = bridge.client.lock().await;
    if guard.as_ref().is_some_and(|held| Arc::ptr_eq(held, lost)) {
        *guard = None;
        let _ = app.emit(CONNECTION_CHANNEL, ConnectionState::Disconnected);
    }
}

/// Connect, forward every daemon event into the webview, and subscribe to all
/// of them. One connection carries every frontend call, so the session's
/// single event-callback slot is claimed exactly once, here — the frontend
/// multiplexes by topic and job id on its side.
async fn connect(app: &AppHandle, auto_spawn: bool) -> Result<Arc<Client>, CallError> {
    let client = Arc::new(Client::connect(auto_spawn).await?);
    let emitter = app.clone();
    client
        .session()
        .set_event_callback(Some(Arc::new(move |event| {
            let _ = emitter.emit(
                EVENT_CHANNEL,
                EventPayload {
                    topic: event.topic.clone(),
                    payload: event.payload.clone(),
                },
            );
        })));
    client
        .session()
        .call::<EventsSubscribe>(&EventsSubscribeParams::default())
        .await?;
    tracing::info!("connected to the daemon");
    let _ = app.emit(CONNECTION_CHANNEL, ConnectionState::Connected);
    Ok(client)
}
