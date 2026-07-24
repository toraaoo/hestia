//! The client SDK's connection core, shared by every domain facade: one
//! persistent, multiplexed connection whose reader task fulfils pending requests
//! by id and delivers events to the installed callback. The typed `call::<C>()`
//! marshals through the contract, so facades stay one-liners and cannot drift.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ipc::errors::IpcError;
use ipc::protocol::{self, Event, Request, Response};
use ipc::{Connection, FrameWriter};
use proto::error::ErrorInfo;
use proto::Contract;
use serde_json::Value;
use tokio::sync::{oneshot, Notify};

/// The default per-call timeout; a wedged handler can't hang the caller forever.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(10);

type EventCallback = Arc<dyn Fn(&Event) + Send + Sync>;

struct Shared {
    pending: Mutex<HashMap<i64, oneshot::Sender<Response>>>,
    event_cb: Mutex<Option<EventCallback>>,
    closed: AtomicBool,
}

pub struct Session {
    shared: Arc<Shared>,
    writer: tokio::sync::Mutex<FrameWriter>,
    next_id: AtomicI64,
    reader: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl Session {
    pub fn new(connection: Connection) -> Self {
        let (mut reader, writer) = connection.into_split();
        let shared = Arc::new(Shared {
            pending: Mutex::new(HashMap::new()),
            event_cb: Mutex::new(None),
            closed: AtomicBool::new(false),
        });
        let reader_shared = shared.clone();
        let handle = tokio::spawn(async move {
            while let Ok(Some(frame)) = reader.recv().await {
                reader_shared.dispatch(&frame);
            }
            reader_shared.close();
        });
        Session {
            shared,
            writer: tokio::sync::Mutex::new(writer),
            next_id: AtomicI64::new(1),
            reader: Mutex::new(Some(handle)),
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
            return Err(IpcError::ConnectionLost);
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.shared.pending.lock().unwrap().insert(id, tx);

        let req = Request::new(channel, payload, Some(id));
        tracing::debug!(channel, id, "call");
        if let Err(e) = self
            .writer
            .lock()
            .await
            .send(&protocol::encode_request(&req))
            .await
        {
            self.shared.pending.lock().unwrap().remove(&id);
            return Err(e);
        }

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(res)) => {
                tracing::debug!(channel, id, ok = res.ok, "call complete");
                Ok(res)
            }
            Ok(Err(_)) => Err(IpcError::ConnectionLost),
            Err(_) => {
                self.shared.pending.lock().unwrap().remove(&id);
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
        *self.shared.event_cb.lock().unwrap() = cb;
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
            state: Mutex<Option<Result<Value, (String, String)>>>,
            notify: Notify,
        }
        let outcome = Arc::new(Outcome {
            state: Mutex::new(None),
            notify: Notify::new(),
        });

        let id_owned = id.to_string();
        let done = done_topic.to_string();
        let error = error_topic.to_string();
        let cb_outcome = outcome.clone();
        self.set_event_callback(Some(Arc::new(move |event: &Event| {
            if event.payload.get("id").and_then(Value::as_str) != Some(id_owned.as_str()) {
                return;
            }
            if event.topic == done {
                *cb_outcome.state.lock().unwrap() = Some(Ok(event.payload.clone()));
                cb_outcome.notify.notify_waiters();
            } else if event.topic == error {
                let msg = event
                    .payload
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let code = event
                    .payload
                    .get("code")
                    .and_then(Value::as_str)
                    .filter(|c| !c.is_empty())
                    .unwrap_or(ipc::errors::HANDLER_ERROR);
                *cb_outcome.state.lock().unwrap() = Some(Err((code.to_string(), msg.to_string())));
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
        state: &Mutex<Option<Result<Value, (String, String)>>>,
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
                return result.map_err(|(code, message)| IpcError::Daemon {
                    code,
                    message,
                    info: Value::Null,
                });
            }
            if self.is_closed() {
                return Err(IpcError::ConnectionLost);
            }
            let _ = tokio::time::timeout(Duration::from_millis(500), notify.notified()).await;
        }
    }
}

impl Shared {
    fn dispatch(&self, frame: &str) {
        let Ok(value) = serde_json::from_str::<Value>(frame) else {
            return; // ignore a malformed frame rather than tear down
        };
        if protocol::is_event(&value) {
            let cb = self.event_cb.lock().unwrap().clone();
            if let Some(cb) = cb {
                cb(&protocol::decode_event(&value));
            }
            return;
        }
        let res = protocol::decode_response(&value);
        let id = res.id.unwrap_or(0);
        if let Some(tx) = self.pending.lock().unwrap().remove(&id) {
            let _ = tx.send(res);
        }
    }

    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        // Wake every waiter so they fail instead of blocking forever.
        self.pending.lock().unwrap().clear();
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(handle) = self.reader.lock().unwrap().take() {
            handle.abort();
        }
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
