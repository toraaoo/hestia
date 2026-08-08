//! Fans daemon events out to subscribed listeners — a socket connection over the
//! `events.subscribe` channel, or an HTTP event stream. Each subscriber holds the
//! sender its transport drains; a closed one is pruned on the next publish.

use std::sync::{Arc, Mutex};

use ipc::protocol::{encode_event, Event};
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

/// Where a delivered event goes. The socket wants a finished frame, because its
/// writer task carries responses in the same channel; a stream wants the event,
/// because SSE frames it differently.
enum Sink {
    Socket(UnboundedSender<String>),
    Stream(UnboundedSender<Arc<Event>>),
}

impl Sink {
    fn is_closed(&self) -> bool {
        match self {
            Sink::Socket(out) => out.is_closed(),
            Sink::Stream(out) => out.is_closed(),
        }
    }

    /// The socket frame is encoded at most once per publish, however many socket
    /// subscribers there are, and not at all when there are none.
    fn send(&self, event: &Arc<Event>, frame: &mut Option<String>) -> bool {
        match self {
            Sink::Socket(out) => {
                let frame = frame.get_or_insert_with(|| encode_event(event));
                out.send(frame.clone()).is_ok()
            }
            Sink::Stream(out) => out.send(event.clone()).is_ok(),
        }
    }
}

struct Sub {
    conn_id: u64,
    sink: Sink,
    // Filter to one id (matched against the event payload's "id"); None
    // subscribes to every event. An entry key also covers the session keys
    // beneath it, so following an entry outlives its processes.
    filter: Option<String>,
    /// The remote key this subscription was opened with, if any. Revoking a key
    /// drops its streams rather than leaving them running until the client
    /// happens to disconnect.
    key: Option<String>,
}

#[derive(Default)]
pub struct EventHub {
    subs: Mutex<Vec<Sub>>,
}

impl EventHub {
    pub fn subscribe(&self, conn_id: u64, out: UnboundedSender<String>, filter: Option<String>) {
        self.add(conn_id, Sink::Socket(out), filter, None);
    }

    /// Subscribe an HTTP event stream. `key` is the remote key it was opened
    /// with, so a revocation can find it again.
    pub fn subscribe_stream(
        &self,
        conn_id: u64,
        out: UnboundedSender<Arc<Event>>,
        filter: Option<String>,
        key: String,
    ) {
        self.add(conn_id, Sink::Stream(out), filter, Some(key));
    }

    fn add(&self, conn_id: u64, sink: Sink, filter: Option<String>, key: Option<String>) {
        tracing::debug!(
            conn = conn_id,
            filter = filter.as_deref(),
            "event subscription added"
        );
        self.subs.lock().unwrap().push(Sub {
            conn_id,
            sink,
            filter,
            key,
        });
    }

    /// Whether any live subscriber takes every event (unfiltered) — only these
    /// receive an idless broadcast like `process.metrics`.
    pub fn has_broadcast_subscriber(&self) -> bool {
        let mut subs = self.subs.lock().unwrap();
        subs.retain(|s| !s.sink.is_closed());
        subs.iter().any(|s| s.filter.is_none())
    }

    pub fn unsubscribe(&self, conn_id: u64) {
        let mut subs = self.subs.lock().unwrap();
        let before = subs.len();
        subs.retain(|s| s.conn_id != conn_id);
        if subs.len() < before {
            tracing::debug!(conn = conn_id, "event subscriptions removed");
        }
    }

    /// Drop every stream opened with a remote key. Revocation is immediate, so a
    /// key that stops opening doors also stops holding the ones already open.
    pub fn drop_key(&self, key_id: &str) {
        let mut subs = self.subs.lock().unwrap();
        let before = subs.len();
        subs.retain(|s| s.key.as_deref() != Some(key_id));
        let dropped = before - subs.len();
        if dropped > 0 {
            tracing::info!(streams = dropped, "dropped streams of a revoked key");
        }
    }

    /// Deliver `event` to every matching subscriber, pruning any whose transport
    /// has gone away.
    pub fn publish(&self, event: &Event) {
        let id = event.payload.get("id").and_then(Value::as_str);
        tracing::trace!(topic = %event.topic, id, "publishing event");
        let event = Arc::new(event.clone());
        let mut frame = None;
        let mut subs = self.subs.lock().unwrap();
        subs.retain(|sub| {
            if let Some(filter) = &sub.filter {
                if !id.is_some_and(|id| proto::naming::process_in_scope(filter, id)) {
                    return !sub.sink.is_closed();
                }
            }
            sub.sink.send(&event, &mut frame)
        });
    }
}

/// The hub as the engine's supervisor sees it: a place to publish, with no
/// knowledge of the transports behind it.
impl engine::ProcessEvents for EventHub {
    fn publish(&self, topic: &str, payload: Value) {
        EventHub::publish(self, &Event::new(topic, payload));
    }
}
