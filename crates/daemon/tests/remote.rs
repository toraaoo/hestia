//! The remote surface end to end: a real `hestiad` on a real TCP port, a key
//! minted the way an operator mints one, and requests made by an ordinary HTTP
//! client that knows nothing about Hestia.
//!
//! This is the layer the in-process tests cannot reach — the config plumbing
//! that opens the listener at all, the misconfiguration refusal, and the frames
//! as they actually cross a socket.
//!
//! Unix-only for the same reason as `e2e.rs`: it drives the daemon over the
//! domain-socket transport. Windows is validated through the win-VM flow.
#![cfg(unix)]

use std::process::{Child, Command};
use std::time::Duration;

use client::proto::remote::Scope;
use client::Client;
use serde_json::Value;

/// A spawned daemon that is stopped and reaped on drop.
struct Node {
    child: Child,
    _home: tempfile::TempDir,
    client: Client,
    /// Where the door actually opened, `host:port`; empty when it did not.
    address: String,
    refusal: String,
}

impl Drop for Node {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Node {
    /// Bring a daemon up with the remote door configured, and report where it
    /// landed. The port is 0 so the OS picks a free one — two tests running at
    /// once must not fight over a number.
    async fn start(settings: &[(&str, Value)]) -> Node {
        let home = tempfile::Builder::new()
            .prefix("hestia-remote-")
            .tempdir()
            .expect("temp dir");
        let sock = home.path().join("hestiad.sock");

        // Written before the daemon starts: the door is opened once, at start,
        // from whatever the settings said then.
        seed(home.path(), settings);

        let child = Command::new(env!("CARGO_BIN_EXE_hestiad"))
            .arg("serve")
            .env("HESTIA_SOCK", &sock)
            .env("HESTIA_HOME", home.path())
            .env("HESTIA_NO_TRAY", "1")
            .env("HESTIA_NO_PRESENCE", "1")
            .spawn()
            .expect("spawn hestiad");

        let mut client = None;
        for _ in 0..100 {
            if let Ok(c) = Client::connect_to(&sock).await {
                if c.app().ping().await.is_ok() {
                    client = Some(c);
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let client = client.expect("daemon did not become reachable within 5s");

        // The listener opens on its own task, so its address may land a moment
        // after the socket answers.
        let mut status = client.remote().status().await.expect("remote.status");
        for _ in 0..50 {
            if !status.address.is_empty() || !status.refusal.is_empty() || !status.enabled {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            status = client.remote().status().await.expect("remote.status");
        }

        Node {
            child,
            _home: home,
            client,
            address: status.address,
            refusal: status.refusal,
        }
    }

    async fn key(&self, scopes: Vec<Scope>) -> String {
        self.client
            .remote()
            .create_key("test", scopes, Vec::new())
            .await
            .expect("mint a key")
            .token
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.address)
    }
}

/// Write a `config.json` the daemon reads at start. The daemon's own
/// `config set` would need it already running, and the door only opens once.
fn seed(home: &std::path::Path, settings: &[(&str, Value)]) {
    let mut remote = serde_json::Map::new();
    for (key, value) in settings {
        remote.insert((*key).to_string(), value.clone());
    }
    let document = serde_json::json!({ "schemaVersion": 1, "remote": remote });
    std::fs::write(
        home.join("config.json"),
        serde_json::to_string_pretty(&document).unwrap(),
    )
    .expect("seed config.json");
}

fn open() -> Vec<(&'static str, Value)> {
    vec![
        ("enabled", Value::Bool(true)),
        ("port", Value::from(0)),
        ("bind", Value::from("127.0.0.1")),
    ]
}

async fn get(url: &str, token: Option<&str>) -> (reqwest::StatusCode, Value) {
    let mut request = reqwest::Client::new().get(url);
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let response = request.send().await.expect("the node answers");
    let status = response.status();
    let body = response.json().await.unwrap_or(Value::Null);
    (status, body)
}

#[tokio::test]
async fn a_node_serves_its_servers_to_a_key_that_holds_the_scope() {
    let node = Node::start(&open()).await;
    assert!(
        !node.address.is_empty(),
        "the door did not open: {}",
        node.refusal
    );
    let token = node.key(vec![Scope::ServerRead]).await;

    let (status, body) = get(&node.url("/api/v1/servers"), Some(&token)).await;

    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(body["success"], Value::Bool(true));
    assert!(body["data"]["servers"].is_array());
}

#[tokio::test]
async fn discovery_answers_without_a_key_and_names_what_it_serves() {
    let node = Node::start(&open()).await;

    let (status, body) = get(&node.url("/api/versions"), None).await;

    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(body["data"]["versions"][0], Value::from("v1"));
    assert_eq!(body["data"]["daemon"], Value::from(common::app::VERSION));
}

#[tokio::test]
async fn a_key_minted_on_one_node_does_not_open_another() {
    let mine = Node::start(&open()).await;
    let theirs = Node::start(&open()).await;
    let token = theirs.key(vec![Scope::ServerRead]).await;

    let (status, _) = get(&mine.url("/api/v1/servers"), Some(&token)).await;

    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn revoking_a_key_over_the_socket_closes_the_door_for_it() {
    let node = Node::start(&open()).await;
    let token = node.key(vec![Scope::ServerRead]).await;
    assert_eq!(
        get(&node.url("/api/v1/servers"), Some(&token)).await.0,
        reqwest::StatusCode::OK
    );

    let listed = node.client.remote().keys().await.expect("list");
    node.client
        .remote()
        .revoke_key(&listed[0].prefix)
        .await
        .expect("revoke");

    assert_eq!(
        get(&node.url("/api/v1/servers"), Some(&token)).await.0,
        reqwest::StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn a_stolen_key_cannot_mint_its_own_replacement() {
    let node = Node::start(&open()).await;
    let token = node.key(Scope::ALL.to_vec()).await;

    // Minting is a socket act by construction, so none of these paths exist.
    for path in [
        "/api/v1/keys",
        "/api/v1/remote/keys",
        "/api/v1/remote/key/create",
    ] {
        let (status, _) = get(&node.url(path), Some(&token)).await;
        assert_eq!(
            status,
            reqwest::StatusCode::NOT_FOUND,
            "{path} is reachable"
        );
    }
    assert_eq!(node.client.remote().keys().await.expect("list").len(), 1);
}

#[tokio::test]
async fn the_door_stays_shut_until_it_is_asked_for() {
    let node = Node::start(&[("enabled", Value::Bool(false))]).await;

    assert!(node.address.is_empty(), "the door opened uninvited");
    assert!(node.refusal.is_empty(), "a closed door is not a refusal");
    // And the socket still serves, so the launcher is unaffected.
    assert!(node.client.app().ping().await.is_ok());
}

/// Docker's 2375-versus-2376 lesson: a door pointed at the world in plaintext
/// does not open, and the daemon still serves its socket so the setting can be
/// corrected without editing JSON by hand.
#[tokio::test]
async fn an_exposed_bind_is_refused_and_the_daemon_carries_on() {
    let node = Node::start(&[
        ("enabled", Value::Bool(true)),
        ("bind", Value::from("0.0.0.0")),
        ("port", Value::from(0)),
    ])
    .await;

    assert!(node.address.is_empty(), "the door opened on 0.0.0.0");
    assert!(
        node.refusal.contains("reverse proxy"),
        "the refusal must name the fix, got: {}",
        node.refusal
    );
    assert!(
        node.client.app().ping().await.is_ok(),
        "a refused door must not take the launcher down with it"
    );
}

#[tokio::test]
async fn an_acknowledged_exposed_bind_opens() {
    let node = Node::start(&[
        ("enabled", Value::Bool(true)),
        ("bind", Value::from("0.0.0.0")),
        ("port", Value::from(0)),
        ("allowInsecure", Value::Bool(true)),
    ])
    .await;

    assert!(
        !node.address.is_empty(),
        "an acknowledged bind should open: {}",
        node.refusal
    );
    let token = node.key(vec![Scope::ServerRead]).await;
    let port = node.address.rsplit(':').next().expect("a port");
    let (status, _) = get(
        &format!("http://127.0.0.1:{port}/api/v1/servers"),
        Some(&token),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
}

#[tokio::test]
async fn a_console_stream_is_an_event_stream() {
    let node = Node::start(&open()).await;
    let token = node.key(vec![Scope::ServerRead]).await;

    let response = reqwest::Client::new()
        .get(node.url("/api/v1/events"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("the node answers");

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("text/event-stream")
    );
}

/// The SDK's channel-to-route table against a real mount. A route the SDK knows
/// and the daemon does not serve is a 404 nobody would see until a front-end
/// tried it, so the two are pinned against each other here.
#[tokio::test]
async fn every_route_the_sdk_knows_is_actually_mounted() {
    let node = Node::start(&open()).await;
    let token = node.key(Scope::ALL.to_vec()).await;
    let http = reqwest::Client::new();

    for route in client::remote::ROUTES {
        let mut path = route.path.to_string();
        for param in route.params {
            path = path.replace(&format!("{{{param}}}"), "probe");
        }
        let url = match route.path {
            "/health" => node.url(&path),
            _ => node.url(&format!("/api/v1{path}")),
        };
        let method = reqwest::Method::from_bytes(route.method.as_str().as_bytes()).unwrap();
        let response = http
            .request(method, &url)
            .bearer_auth(&token)
            .json(&serde_json::json!({ "command": "list", "name": "probe", "value": "probe" }))
            .send()
            .await
            .expect("the node answers");
        let named = format!("{} {}", route.method.as_str(), route.path);
        let status = response.status();
        let body: Value = response.json().await.unwrap_or(Value::Null);

        assert_ne!(
            status,
            reqwest::StatusCode::METHOD_NOT_ALLOWED,
            "{named} is mounted, but not with that method"
        );
        // The probe names a server that does not exist, so most of these are a
        // 404 — but *our* 404 carries the envelope, and an unmounted path does
        // not. That is what tells "no such server" from "no such route".
        assert!(
            body.get("success").is_some(),
            "{named} is not mounted (answered {status} with no envelope)"
        );
    }
}
