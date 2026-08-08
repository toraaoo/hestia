//! The one HTTP client every remote node is reached through. `reqwest::Client`
//! owns a connection pool and is built to be shared: one per node would keep a
//! pool per node and re-handshake TLS on every call.

use std::sync::OnceLock;
use std::time::Duration;

use ipc::errors::IpcError;
use serde_json::Value;

/// Long enough for a restart to finish; short enough that a wedged node fails
/// rather than hanging a front-end. The stream routes never come through here.
const TIMEOUT: Duration = Duration::from_secs(60);

/// How long to wait for a node to answer at all. Separate from [`TIMEOUT`]: an
/// unreachable node should fail a reachability probe quickly.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

pub fn shared() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .user_agent(format!("hestia/{}", common::app::VERSION))
            .build()
            .expect("the shared http client builds from static settings")
    })
}

/// One request to a node, already addressed and authorized.
pub struct Call<'a> {
    pub method: &'a str,
    pub url: String,
    pub token: &'a str,
    /// The JSON body, or the query pairs, depending on the method.
    pub body: Option<Value>,
    pub query: Vec<(String, String)>,
}

/// Send it, and return the status beside whatever JSON came back. Decoding the
/// envelope is the caller's business; this layer only speaks HTTP.
pub async fn send(call: Call<'_>) -> Result<(reqwest::StatusCode, Value), IpcError> {
    let method = reqwest::Method::from_bytes(call.method.as_bytes())
        .map_err(|e| IpcError::Malformed(e.to_string()))?;
    let mut request = shared().request(method, &call.url);
    if !call.token.is_empty() {
        request = request.bearer_auth(call.token);
    }
    if let Some(body) = &call.body {
        request = request.json(body);
    }
    if !call.query.is_empty() {
        request = request.query(&call.query);
    }
    let response = request.send().await.map_err(failed)?;
    let status = response.status();
    Ok((status, response.json().await.unwrap_or(Value::Null)))
}

fn failed(error: reqwest::Error) -> IpcError {
    if error.is_timeout() {
        return IpcError::Timeout("remote".to_string());
    }
    if error.is_connect() {
        return IpcError::ConnectionLost;
    }
    IpcError::Malformed(error.to_string())
}
