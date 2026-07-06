//! Fans daemon events out to subscribed client connections (via the
//! `events.subscribe` channel). Each subscriber holds the sender for its
//! connection's writer task; a closed connection is pruned on the next publish.

use std::sync::Mutex;

use ipc::protocol::{encode_event, Event};
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

struct Sub {
    conn_id: u64,
    out: UnboundedSender<String>,
    // Filter to a single job id (matched against the event payload's "id");
    // None subscribes to every event.
    filter: Option<String>,
}

#[derive(Default)]
pub struct EventHub {
    subs: Mutex<Vec<Sub>>,
}

impl EventHub {
    pub fn subscribe(&self, conn_id: u64, out: UnboundedSender<String>, filter: Option<String>) {
        tracing::debug!(
            conn = conn_id,
            filter = filter.as_deref(),
            "event subscription added"
        );
        self.subs.lock().unwrap().push(Sub {
            conn_id,
            out,
            filter,
        });
    }

    pub fn unsubscribe(&self, conn_id: u64) {
        let mut subs = self.subs.lock().unwrap();
        let before = subs.len();
        subs.retain(|s| s.conn_id != conn_id);
        if subs.len() < before {
            tracing::debug!(conn = conn_id, "event subscriptions removed");
        }
    }

    /// Deliver `event` to every matching subscriber, pruning any whose connection
    /// has gone away.
    pub fn publish(&self, event: &Event) {
        let id = event.payload.get("id").and_then(Value::as_str);
        tracing::trace!(topic = %event.topic, id, "publishing event");
        let frame = encode_event(event);
        let mut subs = self.subs.lock().unwrap();
        subs.retain(|sub| {
            if let Some(filter) = &sub.filter {
                if Some(filter.as_str()) != id {
                    return !sub.out.is_closed();
                }
            }
            sub.out.send(frame.clone()).is_ok()
        });
    }
}
