//! The daemon protocol envelope, layered on the raw frame transport. Both sides
//! encode/decode through here, so the wire format lives in exactly one place.

use serde_json::{json, Value};

/// The protocol major version carried by every envelope. Bump on a breaking wire
/// change; additive fields do not need a bump.
pub const PROTOCOL_VERSION: i64 = 1;

/// Whether a peer advertising major `version` can talk to us. Same major only.
pub fn compatible(version: i64) -> bool {
    version == PROTOCOL_VERSION
}

/// A request: a channel name, a JSON payload, and an optional correlation id.
#[derive(Debug, Clone)]
pub struct Request {
    pub channel: String,
    pub payload: Value,
    pub id: Option<i64>,
    pub version: i64,
}

impl Request {
    pub fn new(channel: impl Into<String>, payload: Value, id: Option<i64>) -> Self {
        Request {
            channel: channel.into(),
            payload,
            id,
            version: PROTOCOL_VERSION,
        }
    }
}

/// A response: success carries a payload; failure carries the daemon's
/// structured error (a serialized `proto::error::ErrorInfo`) as opaque JSON —
/// `ipc` stays domain-free and never interprets it.
#[derive(Debug, Clone)]
pub struct Response {
    pub ok: bool,
    pub payload: Value,
    pub error: Option<Value>,
    pub id: Option<i64>,
    pub version: i64,
}

impl Response {
    pub fn success(payload: Value) -> Self {
        Response {
            ok: true,
            payload,
            error: None,
            id: None,
            version: PROTOCOL_VERSION,
        }
    }

    pub fn failure(error: Value) -> Self {
        Response {
            ok: false,
            payload: Value::Null,
            error: Some(error),
            id: None,
            version: PROTOCOL_VERSION,
        }
    }
}

/// An unsolicited push from the daemon to a subscribed client. It carries no id.
#[derive(Debug, Clone)]
pub struct Event {
    pub topic: String,
    pub payload: Value,
}

pub fn encode_request(req: &Request) -> String {
    let mut j = json!({
        "v": req.version,
        "channel": req.channel,
        "payload": req.payload,
    });
    if let Some(id) = req.id {
        j["id"] = json!(id);
    }
    j.to_string()
}

pub fn encode_response(res: &Response) -> String {
    let mut j = json!({ "v": res.version, "ok": res.ok });
    if res.ok {
        j["payload"] = res.payload.clone();
    } else {
        j["error"] = res.error.clone().unwrap_or_else(|| json!({}));
    }
    if let Some(id) = res.id {
        j["id"] = json!(id);
    }
    j.to_string()
}

pub fn encode_event(event: &Event) -> String {
    json!({ "event": event.topic, "payload": event.payload }).to_string()
}

pub fn decode_request(frame: &str) -> Result<Request, serde_json::Error> {
    let j: Value = serde_json::from_str(frame)?;
    let channel = j
        .get("channel")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let payload = match j.get("payload") {
        Some(p) if !p.is_null() => p.clone(),
        _ => json!({}),
    };
    let id = j.get("id").and_then(Value::as_i64);
    let version = j
        .get("v")
        .and_then(Value::as_i64)
        .unwrap_or(PROTOCOL_VERSION);
    Ok(Request {
        channel,
        payload,
        id,
        version,
    })
}

/// Is this frame an event (an unsolicited push) rather than a response?
pub fn is_event(frame: &Value) -> bool {
    frame.get("event").map(Value::is_string).unwrap_or(false)
}

pub fn decode_response(frame: &Value) -> Response {
    let ok = frame.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let version = frame
        .get("v")
        .and_then(Value::as_i64)
        .unwrap_or(PROTOCOL_VERSION);
    let id = frame.get("id").and_then(Value::as_i64);
    if ok {
        Response {
            ok: true,
            payload: frame.get("payload").cloned().unwrap_or(json!({})),
            error: None,
            id,
            version,
        }
    } else {
        Response {
            ok: false,
            payload: Value::Null,
            error: Some(frame.get("error").cloned().unwrap_or_else(|| json!({}))),
            id,
            version,
        }
    }
}

pub fn decode_event(frame: &Value) -> Event {
    Event {
        topic: frame
            .get("event")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        payload: match frame.get("payload") {
            Some(p) if !p.is_null() => p.clone(),
            _ => json!({}),
        },
    }
}
