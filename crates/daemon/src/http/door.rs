//! The door driven in process: a real `Runtime` over a temp data home, the real
//! service router underneath, the real axum stack on top. Nothing here stubs the
//! layer it checks — a test that mounted its own routes would pass while the
//! allowlist was wrong.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use engine::ServerRecord;
use proto::minecraft::ServerProfile;
use proto::remote::Scope;
use serde_json::{json, Value};
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

    /// A registered, provisioned server. Marked ready because a half-created one
    /// is refused by the guards for reasons that have nothing to do with this
    /// surface.
    fn server(&self, name: &str) -> ServerRecord {
        let servers = self.runtime.engine().servers();
        let record = servers
            .create(
                name,
                ServerProfile {
                    flavor: "vanilla".into(),
                    game_version: "1.21".into(),
                    ..ServerProfile::default()
                },
                None,
            )
            .expect("register a server");
        servers.mark_ready(&record.id).expect("mark ready")
    }

    async fn get(&self, path: &str, token: Option<&str>) -> Response {
        self.send("GET", path, token, None).await
    }

    async fn send(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        json: Option<Value>,
    ) -> Response {
        let mut request = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let body = match json {
            Some(json) => {
                request = request.header(header::CONTENT_TYPE, "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };
        super::app(self.api.clone())
            .oneshot(request.body(body).unwrap())
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
    assert_eq!(body["data"]["kind"], Value::from("entry_not_found"));
    assert_eq!(body["data"]["entry"], Value::from("server"));
    assert_eq!(body["data"]["reference"], Value::from("nosuch"));
}

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

#[tokio::test]
async fn a_read_key_cannot_touch_anything() {
    let node = Node::new();
    let server = node.server("smp");
    let read = node.key(vec![Scope::ServerRead]);
    let id = &server.id;

    let mutations: Vec<(&str, String, Option<Value>)> = vec![
        ("POST", v1(&format!("/servers/{id}/start")), None),
        ("POST", v1(&format!("/servers/{id}/stop")), None),
        ("POST", v1(&format!("/servers/{id}/restart")), None),
        (
            "POST",
            v1(&format!("/servers/{id}/console")),
            Some(json!({ "command": "stop" })),
        ),
        (
            "PATCH",
            v1(&format!("/servers/{id}")),
            Some(json!({ "name": "renamed" })),
        ),
        (
            "PUT",
            v1(&format!("/servers/{id}/config/memory")),
            Some(json!({ "value": "4G" })),
        ),
        ("POST", v1(&format!("/servers/{id}/backups")), None),
        ("DELETE", v1(&format!("/servers/{id}/backups/b1")), None),
        (
            "POST",
            v1(&format!("/servers/{id}/backups/b1/restore")),
            None,
        ),
    ];

    for (method, path, body) in mutations {
        let response = node.send(method, &path, Some(&read), body).await;
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{method} {path} answered a read-only key"
        );
    }
    assert_eq!(
        node.runtime.engine().servers().get(id).unwrap().name,
        "smp",
        "a refused rename must not have landed"
    );
}

#[tokio::test]
async fn each_mutation_costs_its_own_scope() {
    let node = Node::new();
    let server = node.server("smp");
    let id = &server.id;
    let control = node.key(vec![Scope::ServerControl]);
    let write = node.key(vec![Scope::ServerWrite]);
    let backup = node.key(vec![Scope::ServerBackup]);

    // Control drives it and nothing else.
    assert_ne!(
        node.send(
            "POST",
            &v1(&format!("/servers/{id}/stop")),
            Some(&control),
            None
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        node.send(
            "PATCH",
            &v1(&format!("/servers/{id}")),
            Some(&control),
            Some(json!({ "name": "nope" })),
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        node.send(
            "POST",
            &v1(&format!("/servers/{id}/backups")),
            Some(&control),
            None,
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );

    // Write changes it and cannot drive it.
    assert_ne!(
        node.send(
            "PUT",
            &v1(&format!("/servers/{id}/config/memory")),
            Some(&write),
            Some(json!({ "value": "4G" })),
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        node.send(
            "POST",
            &v1(&format!("/servers/{id}/stop")),
            Some(&write),
            None
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );

    // Backup archives it and cannot read the rest.
    assert_eq!(
        node.get(&v1(&format!("/servers/{id}/logs")), Some(&backup))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn a_setting_written_over_http_lands_on_the_record() {
    let node = Node::new();
    let server = node.server("smp");
    let write = node.key(vec![Scope::ServerWrite]);

    let response = node
        .send(
            "PUT",
            &v1(&format!("/servers/{}/config/memory", server.id)),
            Some(&write),
            Some(json!({ "value": "4G" })),
        )
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        node.runtime
            .engine()
            .servers()
            .config_get(&server.id, "memory")
            .unwrap()
            .as_deref(),
        Some("4G")
    );
}

#[tokio::test]
async fn a_rename_over_http_lands_on_the_record() {
    let node = Node::new();
    let server = node.server("smp");
    let write = node.key(vec![Scope::ServerWrite]);

    let response = node
        .send(
            "PATCH",
            &v1(&format!("/servers/{}", server.id)),
            Some(&write),
            Some(json!({ "name": "survival" })),
        )
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        node.runtime
            .engine()
            .servers()
            .get(&server.id)
            .unwrap()
            .name,
        "survival"
    );
}

#[tokio::test]
async fn a_long_operation_answers_202_and_points_at_the_stream() {
    let node = Node::new();
    let server = node.server("smp");
    let backup = node.key(vec![Scope::ServerBackup]);

    let response = node
        .send(
            "POST",
            &v1(&format!("/servers/{}/backups", server.id)),
            Some(&backup),
            None,
        )
        .await;

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(
        response.headers().get(header::LOCATION).unwrap(),
        v1("/events").as_str()
    );
    let body = body(response).await;
    assert!(
        body["data"]["id"].as_str().is_some_and(|id| !id.is_empty()),
        "a 202 must name the job to watch"
    );
}

#[tokio::test]
async fn a_narrowed_key_cannot_mutate_a_server_it_does_not_cover() {
    let node = Node::new();
    let mine = node.server("mine");
    let theirs = node.server("theirs");
    let token = node.key_for(
        vec![Scope::ServerControl, Scope::ServerWrite],
        vec![mine.id.clone()],
    );

    let response = node
        .send(
            "PATCH",
            &v1(&format!("/servers/{}", theirs.id)),
            Some(&token),
            Some(json!({ "name": "hijacked" })),
        )
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        node.runtime
            .engine()
            .servers()
            .get(&theirs.id)
            .unwrap()
            .name,
        "theirs"
    );
}

#[tokio::test]
async fn a_body_bigger_than_the_limit_is_refused_before_a_handler_sees_it() {
    let node = Node::new();
    let server = node.server("smp");
    let write = node.key(vec![Scope::ServerWrite]);

    let response = node
        .send(
            "PATCH",
            &v1(&format!("/servers/{}", server.id)),
            Some(&write),
            Some(json!({ "name": "x".repeat(super::MAX_BODY + 1) })),
        )
        .await;

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        node.runtime
            .engine()
            .servers()
            .get(&server.id)
            .unwrap()
            .name,
        "smp"
    );
}
