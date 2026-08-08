//! The door driven end to end, in process: a real `Runtime` over a temp data
//! home, the real service router underneath, and the real axum stack on top.
//!
//! Nothing here stubs the layer it is checking. A test that mounted its own
//! routes would pass while the allowlist was wrong, which is the one thing this
//! surface cannot afford to get wrong.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use engine::ServerRecord;
use proto::minecraft::ServerProfile;
use proto::remote::Scope;
use serde_json::Value;
use tower::ServiceExt;

use super::{Api, CURRENT};
use crate::runtime::Runtime;
use crate::services::make_router;

/// A daemon's worth of state on a directory that removes itself.
struct Node {
    _home: tempfile::TempDir,
    runtime: Arc<Runtime>,
    api: Api,
}

impl Node {
    fn new() -> Node {
        let home = tempfile::Builder::new()
            .prefix("hestia-http-")
            .tempdir()
            .expect("temp dir");
        let runtime = Arc::new(Runtime::new(
            home.path().join("hestiad.log"),
            Some(home.path()),
        ));
        let api = Api {
            runtime: runtime.clone(),
            router: Arc::new(make_router()),
            throttle: Arc::new(super::throttle::Throttle::default()),
            trusts_proxy: false,
        };
        Node {
            _home: home,
            runtime,
            api,
        }
    }

    /// Mint a key the way `hestia remote key create` does, and hand back the
    /// token — the only moment it exists.
    fn key(&self, scopes: Vec<Scope>) -> String {
        self.key_for(scopes, Vec::new())
    }

    fn key_for(&self, scopes: Vec<Scope>, servers: Vec<String>) -> String {
        self.runtime
            .engine()
            .remote()
            .create("test", scopes, servers)
            .expect("mint")
            .1
    }

    fn server(&self, name: &str) -> ServerRecord {
        self.runtime
            .engine()
            .servers()
            .create(
                name,
                ServerProfile {
                    flavor: "vanilla".into(),
                    game_version: "1.21".into(),
                    ..ServerProfile::default()
                },
                None,
            )
            .expect("register a server")
    }

    async fn get(&self, path: &str, token: Option<&str>) -> Response {
        let mut request = Request::builder().uri(path);
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        super::app(self.api.clone())
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .expect("the router answers")
    }
}

async fn body(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .expect("a body");
    serde_json::from_slice(&bytes).expect("json")
}

fn v1(path: &str) -> String {
    format!("/api/{CURRENT}{path}")
}

// --- discovery -----------------------------------------------------------

#[tokio::test]
async fn discovery_needs_no_key_and_names_both_version_lines() {
    let node = Node::new();

    let response = node.get("/api/versions", None).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = body(response).await;
    assert_eq!(body["success"], Value::Bool(true));
    assert_eq!(body["data"]["versions"][0], Value::from(CURRENT));
    assert_eq!(body["data"]["protocol"], Value::from(ipc::PROTOCOL_VERSION));
    assert_eq!(body["data"]["daemon"], Value::from(common::app::VERSION));
}

#[tokio::test]
async fn every_answer_says_which_api_and_which_daemon_produced_it() {
    let node = Node::new();

    for path in ["/api/versions", "/health", &v1("/servers"), "/nothing-here"] {
        let response = node.get(path, None).await;
        let headers = response.headers();
        assert_eq!(
            headers.get(super::envelope::API_VERSION).unwrap(),
            CURRENT,
            "{path} answered without naming its api version"
        );
        assert!(
            headers.get(super::envelope::DAEMON_VERSION).is_some(),
            "{path} answered without naming the daemon"
        );
    }
}

// --- auth ----------------------------------------------------------------

#[tokio::test]
async fn a_request_with_no_key_is_refused() {
    let node = Node::new();

    let response = node.get(&v1("/servers"), None).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = body(response).await;
    assert_eq!(body["success"], Value::Bool(false));
    assert_eq!(body["code"], Value::from("UNAUTHORIZED"));
    assert_eq!(body["data"]["kind"], Value::from("remote_key_rejected"));
}

#[tokio::test]
async fn a_key_this_node_never_minted_is_refused() {
    let node = Node::new();
    let elsewhere = Node::new().key(vec![Scope::ServerRead]);

    let response = node.get(&v1("/servers"), Some(&elsewhere)).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_revoked_key_stops_working_at_once() {
    let node = Node::new();
    let token = node.key(vec![Scope::ServerRead]);
    assert_eq!(
        node.get(&v1("/servers"), Some(&token)).await.status(),
        StatusCode::OK
    );

    let id = node.runtime.engine().remote().list()[0].id.clone();
    node.runtime.engine().remote().revoke(&id).expect("revoke");

    assert_eq!(
        node.get(&v1("/servers"), Some(&token)).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn a_malformed_authorization_header_is_refused_like_any_other() {
    let node = Node::new();
    let token = node.key(vec![Scope::ServerRead]);

    for header in ["", "   ", "not-a-key", "hst_", &token[..10], "null"] {
        let response = node.get(&v1("/servers"), Some(header)).await;
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{header:?} was accepted"
        );
    }
}

/// The CVE-2026-54593 shape: a valid key, used on a route it was not issued for.
#[tokio::test]
async fn a_valid_key_is_not_permission_for_a_scope_it_does_not_hold() {
    let node = Node::new();
    let backup_only = node.key(vec![Scope::ServerBackup]);

    let response = node.get(&v1("/servers"), Some(&backup_only)).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = body(response).await;
    assert_eq!(body["code"], Value::from("FORBIDDEN"));
    assert_eq!(body["data"]["kind"], Value::from("remote_scope_required"));
    assert_eq!(body["data"]["scope"], Value::from("server:read"));
}

#[tokio::test]
async fn every_read_route_costs_the_read_scope() {
    let node = Node::new();
    let server = node.server("smp");
    let wrong = node.key(vec![Scope::ServerControl]);

    for path in [
        v1("/servers"),
        v1(&format!("/servers/{}", server.id)),
        v1(&format!("/servers/{}/detail", server.id)),
        v1(&format!("/servers/{}/ping", server.id)),
        v1(&format!("/servers/{}/logs", server.id)),
        v1(&format!("/servers/{}/config", server.id)),
        v1(&format!("/servers/{}/content", server.id)),
        v1(&format!("/servers/{}/backups", server.id)),
        v1("/events"),
        v1(&format!("/servers/{}/console", server.id)),
    ] {
        assert_eq!(
            node.get(&path, Some(&wrong)).await.status(),
            StatusCode::FORBIDDEN,
            "{path} answered a key without server:read"
        );
    }
}

// --- the allowlist -------------------------------------------------------

/// The channels 0075 names as never routable. A valid, maximally-scoped key
/// reaches none of them, because there is no path — 404 on an unmounted route,
/// not 403 on a mounted one.
#[tokio::test]
async fn nothing_outside_the_allowlist_has_a_path_at_all() {
    let node = Node::new();
    let every_scope = node.key(Scope::ALL.to_vec());

    for path in [
        v1("/accounts"),
        v1("/skins"),
        v1("/instances"),
        v1("/sync"),
        v1("/update"),
        v1("/config"),
        v1("/keys"),
        v1("/remote/keys"),
        v1("/daemon/stop"),
        "/api/v1/servers/smp/../../accounts".into(),
        "/api/v2/servers".into(),
    ] {
        let response = node.get(&path, Some(&every_scope)).await;
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{path} is reachable and should not be"
        );
    }
}

/// Minting is local by construction: there is no HTTP path to the key channels,
/// so a stolen key cannot mint its own replacement.
#[tokio::test]
async fn a_key_cannot_be_used_to_mint_list_or_revoke_a_key() {
    let node = Node::new();
    let every_scope = node.key(Scope::ALL.to_vec());
    let before = node.runtime.engine().remote().count();

    for path in [
        v1("/remote/key/create"),
        v1("/remote/key/list"),
        v1("/remote/key/revoke"),
        v1("/remote.key.create"),
    ] {
        assert_eq!(
            node.get(&path, Some(&every_scope)).await.status(),
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(node.runtime.engine().remote().count(), before);
}

// --- narrowing -----------------------------------------------------------

#[tokio::test]
async fn a_narrowed_key_never_learns_the_other_servers_exist() {
    let node = Node::new();
    let mine = node.server("mine");
    let theirs = node.server("theirs");
    let token = node.key_for(vec![Scope::ServerRead], vec![mine.id.clone()]);

    let listed = body(node.get(&v1("/servers"), Some(&token)).await).await;
    let names: Vec<&str> = listed["data"]["servers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["mine"]);

    // Refused as absent, never as forbidden: telling the two apart would
    // enumerate every server on the node.
    let response = node
        .get(&v1(&format!("/servers/{}", theirs.id)), Some(&token))
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        body(response).await["data"]["kind"],
        Value::from("entry_not_found")
    );
}

#[tokio::test]
async fn narrowing_is_checked_against_the_id_not_the_name_in_the_path() {
    let node = Node::new();
    let mine = node.server("mine");
    let theirs = node.server("theirs");
    let token = node.key_for(vec![Scope::ServerRead], vec![mine.id.clone()]);

    // A path may name a server by its slug, so a check against the raw
    // reference would be bypassed by using the other spelling.
    assert_eq!(
        node.get(&v1("/servers/mine"), Some(&token)).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        node.get(&v1("/servers/theirs"), Some(&token))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_ne!(theirs.id, "theirs");
}

// --- the envelope --------------------------------------------------------

#[tokio::test]
async fn a_result_arrives_in_the_shape_the_web_sdk_already_reads() {
    let node = Node::new();
    node.server("smp");
    let token = node.key(vec![Scope::ServerRead]);

    let response = node.get(&v1("/servers"), Some(&token)).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/json"
    );
    let body = body(response).await;
    assert_eq!(body["success"], Value::Bool(true));
    assert_eq!(body["message"], Value::from("1 servers"));
    assert!(body.get("code").is_none(), "a success carries no code");
    assert_eq!(body["data"]["servers"][0]["name"], Value::from("smp"));
}

#[tokio::test]
async fn a_missing_server_is_the_daemons_own_error_under_an_http_status() {
    let node = Node::new();
    let token = node.key(vec![Scope::ServerRead]);

    let response = node.get(&v1("/servers/nosuch"), Some(&token)).await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = body(response).await;
    assert_eq!(body["code"], Value::from("NOT_FOUND"));
    // The coarse code is for a generic integrator; the whole ErrorInfo is still
    // there for a client that localizes from `kind`.
    assert_eq!(body["data"]["kind"], Value::from("entry_not_found"));
    assert_eq!(body["data"]["entry"], Value::from("server"));
    assert_eq!(body["data"]["reference"], Value::from("nosuch"));
}

// --- streams -------------------------------------------------------------

#[tokio::test]
async fn a_stream_answers_as_an_event_stream() {
    let node = Node::new();
    let token = node.key(vec![Scope::ServerRead]);

    let response = node.get(&v1("/events"), Some(&token)).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );
}

#[tokio::test]
async fn a_console_stream_needs_a_server_the_key_can_reach() {
    let node = Node::new();
    let mine = node.server("mine");
    let theirs = node.server("theirs");
    let token = node.key_for(vec![Scope::ServerRead], vec![mine.id.clone()]);

    assert_eq!(
        node.get(&v1(&format!("/servers/{}/console", mine.id)), Some(&token))
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        node.get(
            &v1(&format!("/servers/{}/console", theirs.id)),
            Some(&token)
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
}

// --- rate limiting -------------------------------------------------------

#[tokio::test]
async fn guessing_runs_out_of_attempts_and_says_when_to_come_back() {
    let node = Node::new();

    let mut seen_429 = false;
    for attempt in 0..30 {
        let response = node.get(&v1("/servers"), Some("hst_wrongwrongwrong")).await;
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            let retry = response
                .headers()
                .get(header::RETRY_AFTER)
                .expect("a 429 says when to come back");
            assert!(retry.to_str().unwrap().parse::<u32>().unwrap() > 0);
            assert_eq!(
                body(response).await["code"],
                Value::from("TOO_MANY_REQUESTS")
            );
            seen_429 = true;
            break;
        }
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "attempt {attempt} answered something other than a refusal"
        );
    }
    assert!(seen_429, "the failure path is not rate-limited");
}

#[tokio::test]
async fn a_key_that_works_never_spends_the_budget() {
    let node = Node::new();
    let token = node.key(vec![Scope::ServerRead]);

    for _ in 0..40 {
        assert_eq!(
            node.get(&v1("/servers"), Some(&token)).await.status(),
            StatusCode::OK
        );
    }
}
