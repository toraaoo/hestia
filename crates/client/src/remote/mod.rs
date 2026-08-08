//! The SDK's second transport: a node reached over HTTP instead of a socket.
//!
//! A caller marshals through the same `Contract` either way. This turns the
//! contract's channel into the route that serves it, unwraps the HTTP envelope
//! back into the daemon's own `ErrorInfo`, and reports failures as the same
//! [`IpcError`] a socket call would.

mod routes;

use std::time::Duration;

use ipc::errors::{self, IpcError};
use proto::error::ErrorInfo;
use proto::Contract;
use serde_json::{Map, Value};

pub use routes::{Route, ROUTES};

/// A remote node and the key that opens it.
pub struct Node {
    http: reqwest::Client,
    /// The node's origin, no trailing slash.
    base: String,
    token: String,
}

impl Node {
    pub fn new(url: &str, token: &str) -> Result<Node, IpcError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| IpcError::Malformed(e.to_string()))?;
        Ok(Node {
            http,
            base: url.trim_end_matches('/').to_string(),
            token: token.to_string(),
        })
    }

    /// What the node says it is, without spending a key on it.
    pub async fn versions(&self) -> Result<Value, IpcError> {
        let response = self
            .http
            .get(format!("{}/api/versions", self.base))
            .send()
            .await
            .map_err(transport)?;
        let body: Value = response.json().await.map_err(transport)?;
        Ok(body.get("data").cloned().unwrap_or(Value::Null))
    }

    pub async fn call<C: Contract>(&self, params: &C::Params) -> Result<C::Result, IpcError> {
        let payload = serde_json::to_value(params).unwrap_or(Value::Null);
        let value = self.call_raw(C::CHANNEL, payload).await?;
        serde_json::from_value(value).map_err(|e| IpcError::Malformed(e.to_string()))
    }

    /// Dispatch by channel name — what the desktop bridge forwards, having no
    /// static type for the payload it is carrying.
    pub async fn call_raw(&self, channel: &str, payload: Value) -> Result<Value, IpcError> {
        let route = routes::of(channel).ok_or_else(|| IpcError::Daemon {
            code: errors::UNKNOWN_CHANNEL.to_string(),
            message: format!("'{channel}' is not reachable on a remote node"),
            info: serde_json::to_value(ErrorInfo::UnknownChannel {
                channel: channel.to_string(),
            })
            .unwrap_or(Value::Null),
        })?;

        let mut fields = match payload {
            Value::Object(map) => map,
            _ => Map::new(),
        };
        let path = fill(route, &mut fields)?;
        let url = format!("{}/api/{}{path}", self.base, super::remote::API);

        let mut request = self
            .http
            .request(
                reqwest::Method::from_bytes(route.method.as_str().as_bytes())
                    .map_err(|e| IpcError::Malformed(e.to_string()))?,
                url,
            )
            .bearer_auth(&self.token);
        if route.method.takes_a_body() {
            request = request.json(&Value::Object(fields));
        } else {
            let query: Vec<(String, String)> = fields
                .into_iter()
                .filter(|(_, value)| !value.is_null())
                .map(|(key, value)| (key, scalar(&value)))
                .collect();
            request = request.query(&query);
        }

        let response = request.send().await.map_err(transport)?;
        let status = response.status();
        let body: Value = response.json().await.unwrap_or(Value::Null);
        if status.is_success() {
            return Ok(body.get("data").cloned().unwrap_or(Value::Null));
        }
        Err(failure(&body))
    }
}

/// Which API major this build speaks. Its own constant rather than the socket's
/// `PROTOCOL_VERSION`: the two contracts change for different reasons.
pub const API: &str = "v1";

/// Spend the named payload fields on the path's placeholders. A field the path
/// needs and the payload does not carry is the caller's mistake, not the node's.
fn fill(route: &Route, fields: &mut Map<String, Value>) -> Result<String, IpcError> {
    let mut path = route.path.to_string();
    for name in route.params {
        let value = fields.remove(*name).unwrap_or(Value::Null);
        let filled = scalar(&value);
        if filled.is_empty() {
            return Err(IpcError::Malformed(format!(
                "{} needs a '{name}' to address a route",
                route.channel
            )));
        }
        path = path.replace(&format!("{{{name}}}"), &urlencode(&filled));
    }
    Ok(path)
}

/// A payload field as a path or query segment. Strings pass through unquoted;
/// anything else takes its JSON form.
fn scalar(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Percent-encode everything outside the unreserved set, so a server named with
/// a slash cannot reach a route it was not addressed to.
fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Rebuild the daemon's own typed failure from the envelope's `data`, so a
/// caller matches on the same `ErrorInfo` it would have got over the socket.
fn failure(body: &Value) -> IpcError {
    let raw = body.get("data").cloned().unwrap_or(Value::Null);
    let info = serde_json::from_value::<ErrorInfo>(raw.clone()).unwrap_or(ErrorInfo::Internal {
        detail: body
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("the node answered a failure this build cannot read")
            .to_string(),
    });
    IpcError::Daemon {
        code: info.code().to_string(),
        message: info.to_string(),
        info: raw,
    }
}

fn transport(error: reqwest::Error) -> IpcError {
    if error.is_timeout() {
        return IpcError::Timeout("remote".to_string());
    }
    if error.is_connect() {
        return IpcError::ConnectionLost;
    }
    IpcError::Malformed(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fields(value: Value) -> Map<String, Value> {
        match value {
            Value::Object(map) => map,
            _ => Map::new(),
        }
    }

    #[test]
    fn a_path_takes_its_placeholders_from_the_payload() {
        let route = routes::of("server.status").expect("routed");
        let mut payload = fields(json!({ "server": "smp" }));
        assert_eq!(fill(route, &mut payload).unwrap(), "/servers/smp");
        assert!(
            payload.is_empty(),
            "a spent field must not repeat in a body"
        );
    }

    #[test]
    fn a_multi_placeholder_path_spends_every_field_it_names() {
        let route = routes::of("server.backup.restore").expect("routed");
        let mut payload = fields(json!({ "server": "smp", "backup": "b7", "id": "" }));
        assert_eq!(
            fill(route, &mut payload).unwrap(),
            "/servers/smp/backups/b7/restore"
        );
        assert_eq!(payload.keys().collect::<Vec<_>>(), ["id"]);
    }

    #[test]
    fn a_reference_that_would_escape_its_route_is_encoded() {
        let route = routes::of("server.status").expect("routed");
        let mut payload = fields(json!({ "server": "../../accounts" }));
        let path = fill(route, &mut payload).unwrap();
        assert_eq!(path, "/servers/..%2F..%2Faccounts");
        assert!(!path.contains("/../"), "a traversal reached the path");
    }

    #[test]
    fn a_missing_reference_is_refused_before_anything_is_sent() {
        let route = routes::of("server.status").expect("routed");
        let mut payload = fields(json!({}));
        assert!(matches!(
            fill(route, &mut payload),
            Err(IpcError::Malformed(_))
        ));
    }

    #[test]
    fn a_failure_arrives_as_the_daemons_own_typed_error() {
        let body = json!({
            "success": false,
            "message": "no server matches 'smp'",
            "code": "NOT_FOUND",
            "data": { "kind": "entry_not_found", "entry": "server", "reference": "smp" },
        });
        match failure(&body) {
            IpcError::Daemon { code, info, .. } => {
                assert_eq!(code, errors::NOT_FOUND);
                let parsed: ErrorInfo = serde_json::from_value(info).expect("typed");
                assert!(matches!(parsed, ErrorInfo::EntryNotFound { .. }));
            }
            other => panic!("expected a daemon failure, got {other:?}"),
        }
    }

    #[test]
    fn a_failure_with_no_structured_error_still_reports_something() {
        let body = json!({ "success": false, "message": "gateway said no" });
        match failure(&body) {
            IpcError::Daemon { message, .. } => assert!(message.contains("gateway said no")),
            other => panic!("expected a daemon failure, got {other:?}"),
        }
    }
}
