//! The client SDK's connection core, shared by every domain facade: one
//! persistent, multiplexed connection whose reader task fulfils pending requests
//! by id and delivers events to the installed callback. The typed `call::<C>()`
//! marshals through the contract, so facades stay one-liners and cannot drift.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ipc::errors::IpcError;
use ipc::protocol::{self, DecodeError, Event, Request, Response};
use ipc::{Connection, FrameWriter};
use proto::error::ErrorInfo;
use proto::Contract;
use serde_json::Value;
use tokio::sync::{oneshot, Notify};

/// The default per-call timeout; a wedged handler can't hang the caller forever.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(10);

type EventCallback = Arc<dyn Fn(&Event) + Send + Sync>;

/// How a job ended, other than well. Cancellation is kept apart from failure
/// because nothing went wrong: a front-end must be able to say "cancelled"
/// rather than render it as an error.
enum Terminal {
    Cancelled,
    Failed(ErrorInfo),
}

impl Terminal {
    fn into_error(self) -> IpcError {
        match self {
            Terminal::Cancelled => IpcError::Cancelled,
            Terminal::Failed(info) => IpcError::Daemon {
                code: info.code().to_string(),
                message: info.to_string(),
                info: serde_json::to_value(&info).unwrap_or(Value::Null),
            },
        }
    }
}

#[derive(Default)]
struct Shared {
    pending: Mutex<HashMap<i64, oneshot::Sender<Response>>>,
    event_cb: Mutex<Option<EventCallback>>,
    closed: AtomicBool,
    /// Set when the connection was torn down for a protocol-version mismatch, so
    /// a woken waiter reports why rather than a bare `ConnectionLost`.
    mismatch: Mutex<Option<(i64, i64)>>,
}

/// How a session reaches its daemon. Every facade above this is written once and
/// works either way: the contract is the same, only the wire differs.
enum Transport {
    /// The local daemon, over its socket.
    Socket {
        writer: tokio::sync::Mutex<FrameWriter>,
        next_id: AtomicI64,
        reader: Mutex<Option<tokio::task::JoinHandle<()>>>,
    },
    /// A remote node, over HTTP.
    #[cfg(feature = "remote")]
    Remote {
        node: Arc<crate::remote::Node>,
        /// The event stream, opened on the first subscription rather than at
        /// connect: a one-shot call has no use for one.
        stream: Mutex<Option<tokio::task::JoinHandle<()>>>,
    },
}

pub struct Session {
    shared: Arc<Shared>,
    transport: Transport,
}

impl Session {
    pub fn new(connection: Connection) -> Self {
        let (mut reader, writer) = connection.into_split();
        let shared = Arc::new(Shared::default());
        let reader_shared = shared.clone();
        tracing::trace!("session opened");
        let handle = tokio::spawn(async move {
            while let Ok(Some(frame)) = reader.recv().await {
                reader_shared.dispatch(&frame);
            }
            reader_shared.close();
        });
        Session {
            shared,
            transport: Transport::Socket {
                writer: tokio::sync::Mutex::new(writer),
                next_id: AtomicI64::new(1),
                reader: Mutex::new(Some(handle)),
            },
        }
    }

    /// Drive a remote node through the same contracts.
    #[cfg(feature = "remote")]
    pub fn remote(node: crate::remote::Node) -> Self {
        tracing::trace!("remote session opened");
        Session {
            shared: Arc::new(Shared::default()),
            transport: Transport::Remote {
                node: Arc::new(node),
                stream: Mutex::new(None),
            },
        }
    }

    pub fn is_closed(&self) -> bool {
        self.shared.closed.load(Ordering::SeqCst)
    }

    /// Raw request; errors only on transport failure (a daemon-side error is a
    /// `Response` with `ok == false`).
    pub async fn call_raw(
        &self,
        channel: &str,
        payload: Value,
        timeout: Duration,
    ) -> Result<Response, IpcError> {
        if self.is_closed() {
            return Err(self.shared.close_error());
        }
        let (writer, next_id) = match &self.transport {
            Transport::Socket {
                writer, next_id, ..
            } => (writer, next_id),
            #[cfg(feature = "remote")]
            Transport::Remote { node, .. } => {
                return remote_call(node, channel, payload, timeout).await
            }
        };
        let id = next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.shared.pending.lock().unwrap().insert(id, tx);

        let req = Request::new(channel, payload, Some(id));
        tracing::debug!(channel, id, "call");
        let frame = protocol::encode_request(&req);
        // The wire is what a client can show that the daemon's own logs cannot:
        // which frames this process sent, correlated, sized and timed. Payloads
        // are never logged — they carry access tokens and rcon passwords — so
        // only the byte count goes out.
        tracing::trace!(
            direction = "send",
            channel,
            id,
            bytes = frame.len(),
            "frame"
        );
        let started = std::time::Instant::now();
        if let Err(e) = writer.lock().await.send(&frame).await {
            self.shared.pending.lock().unwrap().remove(&id);
            tracing::trace!(channel, id, error = %e, "send failed");
            return Err(e);
        }

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(res)) => {
                tracing::debug!(channel, id, ok = res.ok, "call complete");
                tracing::trace!(
                    direction = "recv",
                    channel,
                    id,
                    ok = res.ok,
                    elapsed_ms = started.elapsed().as_millis(),
                    "frame"
                );
                Ok(res)
            }
            Ok(Err(_)) => Err(self.shared.close_error()),
            Err(_) => {
                self.shared.pending.lock().unwrap().remove(&id);
                tracing::trace!(
                    channel,
                    id,
                    elapsed_ms = started.elapsed().as_millis(),
                    "timed out waiting for a reply"
                );
                Err(IpcError::Timeout(channel.to_string()))
            }
        }
    }

    /// Send `C::Params` over `C::CHANNEL` and decode `C::Result`, mapping a
    /// daemon-side error to `IpcError::Daemon`.
    pub async fn call<C: Contract>(&self, params: &C::Params) -> Result<C::Result, IpcError> {
        self.call_with_timeout::<C>(params, CALL_TIMEOUT).await
    }

    pub async fn call_with_timeout<C: Contract>(
        &self,
        params: &C::Params,
        timeout: Duration,
    ) -> Result<C::Result, IpcError> {
        let payload =
            serde_json::to_value(params).map_err(|e| IpcError::Malformed(e.to_string()))?;
        let res = self.call_raw(C::CHANNEL, payload, timeout).await?;
        let res = must(res)?;
        serde_json::from_value(res.payload).map_err(|e| IpcError::Malformed(e.to_string()))
    }

    /// Like `call`, but a `not_found` error becomes `Ok(None)` instead of an error.
    pub async fn try_call<C: Contract>(
        &self,
        params: &C::Params,
    ) -> Result<Option<C::Result>, IpcError> {
        let payload =
            serde_json::to_value(params).map_err(|e| IpcError::Malformed(e.to_string()))?;
        let res = self.call_raw(C::CHANNEL, payload, CALL_TIMEOUT).await?;
        let res = match must(res) {
            Err(IpcError::Daemon { code, .. }) if code == ipc::errors::NOT_FOUND => {
                return Ok(None)
            }
            other => other?,
        };
        serde_json::from_value(res.payload)
            .map(Some)
            .map_err(|e| IpcError::Malformed(e.to_string()))
    }

    pub fn set_event_callback(&self, cb: Option<EventCallback>) {
        let wanted = cb.is_some();
        *self.shared.event_cb.lock().unwrap() = cb;
        if wanted {
            self.open_event_stream();
        }
    }

    /// A socket session already has its reader; a remote one opens an SSE stream
    /// the first time anybody asks for events.
    fn open_event_stream(&self) {
        #[cfg(feature = "remote")]
        if let Transport::Remote { node, stream } = &self.transport {
            let mut held = stream.lock().unwrap();
            if held.as_ref().is_some_and(|task| !task.is_finished()) {
                return;
            }
            let node = node.clone();
            let shared = self.shared.clone();
            *held = Some(tokio::spawn(async move {
                node.follow(|event| {
                    let cb = shared.event_cb.lock().unwrap().clone();
                    if let Some(cb) = cb {
                        cb(event);
                    }
                })
                .await;
            }));
        }
    }

    /// Subscribe to `id`'s events, run `start`, and block until the done or error
    /// topic arrives, handing every other matching event to `on_event`. Returns
    /// the done event's payload; errors with the error event's message.
    pub async fn run_job<F, Fut>(
        &self,
        id: &str,
        done_topic: &str,
        error_topic: &str,
        on_event: impl Fn(&Event) + Send + Sync + 'static,
        start: F,
    ) -> Result<Value, IpcError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), IpcError>>,
    {
        struct Outcome {
            state: Mutex<Option<Result<Value, Terminal>>>,
            notify: Notify,
        }
        let outcome = Arc::new(Outcome {
            state: Mutex::new(None),
            notify: Notify::new(),
        });

        let id_owned = id.to_string();
        let done = done_topic.to_string();
        let error = error_topic.to_string();
        // Every job family's terminal topics share one prefix
        // (`<family>.done|error|cancelled`), so the third is derived rather than
        // threaded through all seven facades — and `tests/wire.rs` fails the
        // build if a family ever names it otherwise.
        let cancelled = done.replace(".done", ".cancelled");
        let cb_outcome = outcome.clone();
        self.set_event_callback(Some(Arc::new(move |event: &Event| {
            if event.payload.get("id").and_then(Value::as_str) != Some(id_owned.as_str()) {
                return;
            }
            if event.topic == done {
                *cb_outcome.state.lock().unwrap() = Some(Ok(event.payload.clone()));
                cb_outcome.notify.notify_waiters();
            } else if event.topic == cancelled {
                *cb_outcome.state.lock().unwrap() = Some(Err(Terminal::Cancelled));
                cb_outcome.notify.notify_waiters();
            } else if event.topic == error {
                let info = event
                    .payload
                    .get("error")
                    .and_then(|e| serde_json::from_value::<ErrorInfo>(e.clone()).ok())
                    .unwrap_or_else(|| ErrorInfo::Internal {
                        detail: "the job failed".into(),
                    });
                *cb_outcome.state.lock().unwrap() = Some(Err(Terminal::Failed(info)));
                cb_outcome.notify.notify_waiters();
            } else {
                on_event(event);
            }
        })));

        let result = self
            .run_job_inner(id, start, &outcome.state, &outcome.notify)
            .await;
        self.set_event_callback(None);
        result
    }

    async fn run_job_inner<F, Fut>(
        &self,
        id: &str,
        start: F,
        state: &Mutex<Option<Result<Value, Terminal>>>,
        notify: &Notify,
    ) -> Result<Value, IpcError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), IpcError>>,
    {
        use proto::events::{EventsSubscribe, EventsSubscribeParams};
        self.call::<EventsSubscribe>(&EventsSubscribeParams { id: id.to_string() })
            .await?;
        start().await?;

        loop {
            if let Some(result) = state.lock().unwrap().take() {
                return result.map_err(Terminal::into_error);
            }
            if self.is_closed() {
                return Err(self.shared.close_error());
            }
            let _ = tokio::time::timeout(Duration::from_millis(500), notify.notified()).await;
        }
    }
}

impl Shared {
    fn dispatch(&self, frame: &str) {
        let Ok(value) = serde_json::from_str::<Value>(frame) else {
            tracing::trace!(bytes = frame.len(), "ignoring a malformed frame");
            return; // ignore a malformed frame rather than tear down
        };
        if protocol::is_event(&value) {
            match protocol::decode_event(&value) {
                Ok(event) => {
                    // Unsolicited, so it correlates with no call: the topic is
                    // what a reader needs to see a job's progress arriving (or
                    // not).
                    tracing::trace!(
                        direction = "recv",
                        topic = %event.topic,
                        bytes = frame.len(),
                        "event"
                    );
                    let cb = self.event_cb.lock().unwrap().clone();
                    if let Some(cb) = cb {
                        cb(&event);
                    }
                }
                Err(e) => self.refuse(e),
            }
            return;
        }
        match protocol::decode_response(&value) {
            Ok(res) => {
                let id = res.id.unwrap_or(0);
                if let Some(tx) = self.pending.lock().unwrap().remove(&id) {
                    let _ = tx.send(res);
                }
            }
            Err(e) => self.refuse(e),
        }
    }

    /// What an undecodable frame costs the connection. A foreign-major daemon is
    /// refused, not silently consumed: tear the connection down so every waiter
    /// fails fast rather than timing out, recording the mismatch so they report
    /// why. A junk frame is ignored — one bad frame is not a reason to drop an
    /// otherwise-compatible connection.
    fn refuse(&self, error: DecodeError) {
        match error {
            DecodeError::IncompatibleVersion { got, want } => {
                tracing::warn!(
                    got,
                    want,
                    "closing connection: incompatible daemon protocol version"
                );
                *self.mismatch.lock().unwrap() = Some((got, want));
                self.close();
            }
            DecodeError::Malformed(_) => {}
        }
    }

    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        // Wake every waiter so they fail instead of blocking forever.
        let waiting = self.pending.lock().unwrap().drain().count();
        tracing::trace!(waiting, "connection closed");
    }

    /// The error a waiter woken by a torn-down connection should report: a
    /// version mismatch names itself, everything else is a bare disconnect.
    fn close_error(&self) -> IpcError {
        match *self.mismatch.lock().unwrap() {
            Some((got, want)) => IpcError::IncompatibleVersion { got, want },
            None => IpcError::ConnectionLost,
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        match &self.transport {
            Transport::Socket { reader, .. } => {
                if let Some(handle) = reader.lock().unwrap().take() {
                    handle.abort();
                }
            }
            #[cfg(feature = "remote")]
            Transport::Remote { stream, .. } => {
                if let Some(handle) = stream.lock().unwrap().take() {
                    handle.abort();
                }
            }
        }
    }
}

/// A remote call, shaped back into the `Response` a socket would have produced —
/// so `must`, `try_call` and every facade above are unchanged.
#[cfg(feature = "remote")]
async fn remote_call(
    node: &crate::remote::Node,
    channel: &str,
    payload: Value,
    timeout: Duration,
) -> Result<Response, IpcError> {
    match tokio::time::timeout(timeout, node.call_raw(channel, payload)).await {
        Ok(Ok(data)) => Ok(Response::success(data)),
        Ok(Err(IpcError::Daemon { info, .. })) => Ok(Response::failure(info)),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(IpcError::Timeout(channel.to_string())),
    }
}

/// Turn a daemon-side error `Response` into an `IpcError`; otherwise hand it back.
pub fn must(res: Response) -> Result<Response, IpcError> {
    if res.ok {
        return Ok(res);
    }
    let raw = res.error.unwrap_or(Value::Null);
    let info =
        serde_json::from_value::<ErrorInfo>(raw.clone()).unwrap_or_else(|_| ErrorInfo::Internal {
            detail: "daemon error".into(),
        });
    Err(IpcError::Daemon {
        code: info.code().into(),
        message: info.to_string(),
        info: raw,
    })
}

/// A client-generated job id lets callers subscribe before starting a job, so
/// even one that finishes instantly cannot slip its terminal event past us.
pub fn job_id(prefix: &str) -> String {
    use std::sync::atomic::AtomicU64;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    let pid = std::process::id();
    format!("{prefix}-{pid}-{n}")
}
