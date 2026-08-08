//! The second front door: HTTP onto the router the socket already uses
//! ([0072](../../../../docs/decisions/0072-http-is-a-second-door.md)). A route
//! serializes a contract's params, dispatches on its channel, and re-envelopes
//! the answer; no handler is duplicated.
//!
//! The mount table is the security boundary — an unlisted channel has no path,
//! so adding one to `services/` never widens this surface
//! ([0075](../../../../docs/decisions/0075-the-remote-surface-is-an-allowlist.md)).

mod auth;
#[cfg(test)]
mod door;
mod envelope;
mod routes;
mod status;
mod stream;
mod throttle;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use ipc::protocol::Request;
use ipc::Peer;
use proto::error::ErrorInfo;
use proto::Contract;
use serde_json::Value;
use tokio::net::TcpListener;
use tower_http::limit::RequestBodyLimitLayer;

use crate::runtime::{Door, HandlerContext, Router, Runtime};

/// The API majors this build serves, oldest first. A breaking change mounts a
/// new one *beside* the old rather than replacing it, so both serve through a
/// deprecation window ([0073](../../../../docs/decisions/0073-the-http-surface-is-versioned-in-the-path.md)).
pub const VERSIONS: &[&str] = &["v1"];

/// The newest of [`VERSIONS`], for the response header.
pub const CURRENT: &str = "v1";

/// No route takes a body worth more than this. A server config is the largest
/// thing that arrives, and it is kilobytes.
const MAX_BODY: usize = 256 * 1024;

/// A plain call has finished or failed long before this. The stream routes opt
/// out — a `text/event-stream` is supposed to stay open.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Everything a route needs: the runtime behind the engine, the one router both
/// doors dispatch through, and the budget an address has for getting the key
/// wrong.
#[derive(Clone)]
pub struct Api {
    runtime: Arc<Runtime>,
    router: Arc<Router>,
    throttle: Arc<throttle::Throttle>,
    /// Whether an `X-Forwarded-For` may be believed. Read once at start: a
    /// proxy is part of a deployment, not something that changes mid-flight.
    trusts_proxy: bool,
}

impl Api {
    fn new(runtime: Arc<Runtime>, router: Arc<Router>) -> Api {
        let trusts_proxy = runtime.engine().config().settings().remote.trusted_proxy;
        Api {
            runtime,
            router,
            throttle: Arc::new(throttle::Throttle::default()),
            trusts_proxy,
        }
    }

    /// Serialize `C::Params`, dispatch on `C::CHANNEL` through the router the
    /// socket uses, and decode `C::Result` back out.
    async fn call<C: Contract>(&self, params: C::Params) -> Result<C::Result, ErrorInfo> {
        let payload = serde_json::to_value(params).unwrap_or(Value::Null);
        // No mounted channel writes to `out` — `events.subscribe` is not a
        // route — so nothing is lost by dropping the receiver.
        let (out, _drain) = tokio::sync::mpsc::unbounded_channel();
        let ctx = HandlerContext {
            runtime: self.runtime.clone(),
            conn_id: next_conn_id(),
            out,
            peer: Peer::remote(),
        };
        let response = self
            .router
            .route(Request::new(C::CHANNEL, payload, None), ctx)
            .await;
        if response.ok {
            return serde_json::from_value(response.payload).map_err(|e| ErrorInfo::Internal {
                detail: format!(
                    "{} answered a payload this build cannot read: {e}",
                    C::CHANNEL
                ),
            });
        }
        let raw = response.error.unwrap_or(Value::Null);
        Err(
            serde_json::from_value(raw).unwrap_or_else(|_| ErrorInfo::Internal {
                detail: format!("{} failed without a structured error", C::CHANNEL),
            }),
        )
    }

    /// Resolve the `{id}` in a path to a server id, refusing anything this key
    /// was not issued for.
    ///
    /// A narrowed key gets **404** for the servers it does not cover, never 403:
    /// telling "not yours" from "not there" enumerates the node. The check is
    /// against the resolved id, since a path may name a server by its slug.
    fn server(&self, grant: &engine::Grant, reference: &str) -> Result<String, envelope::Failure> {
        self.runtime
            .engine()
            .servers()
            .get(reference)
            .filter(|record| grant.covers(&record.id))
            .map(|record| record.id)
            .ok_or_else(|| {
                ErrorInfo::EntryNotFound {
                    entry: proto::error::EntryKind::Server,
                    reference: reference.to_string(),
                }
                .into()
            })
    }
}

/// Connection ids, shared with the socket so a log line's `conn` is unambiguous.
fn next_conn_id() -> u64 {
    // Counts down; the socket's counter counts up, so the two never collide.
    static COUNTER: AtomicU64 = AtomicU64::new(u64::MAX);
    COUNTER.fetch_sub(1, Ordering::Relaxed)
}

/// As much of a presented credential as may be written down: the readable head,
/// which identifies a key without being one.
fn redact(presented: &str) -> &str {
    match presented.len() {
        0 => "<none>",
        1..=12 => "<malformed>",
        _ => &presented[..12],
    }
}

/// Open the door in the background, if it is configured to be open at all.
pub fn spawn(runtime: Arc<Runtime>, router: Arc<Router>) {
    tokio::spawn(serve(runtime, router));
}

/// Open the door, if it is configured to be open at all.
///
/// A refusal takes down the listener and nothing else: the daemon keeps serving
/// its socket, so the misconfiguration is fixable with `hestia config set`
/// rather than by editing JSON by hand.
async fn serve(runtime: Arc<Runtime>, router: Arc<Router>) {
    let remote = runtime.engine().config().settings().remote;
    if !remote.enabled {
        tracing::debug!("the remote surface is off (remote.enabled)");
        return;
    }
    if let Err(refusal) = remote.admissible() {
        tracing::error!("refusing to open the remote surface: {refusal}");
        runtime.set_remote_door(Door {
            refusal: refusal.to_string(),
            ..Door::default()
        });
        return;
    }

    let address = remote.address();
    let listener = match TcpListener::bind(&address).await {
        Ok(listener) => listener,
        Err(e) => {
            let refusal = format!("cannot bind {address}: {e}");
            tracing::error!("{refusal}");
            runtime.set_remote_door(Door {
                refusal,
                ..Door::default()
            });
            return;
        }
    };
    let bound = listener
        .local_addr()
        .map_or_else(|_| address.clone(), |addr| addr.to_string());
    tracing::info!(
        keys = runtime.engine().remote().count(),
        "remote surface listening on http://{bound}"
    );
    runtime.set_remote_door(Door {
        address: bound,
        refusal: String::new(),
    });
    if let Err(e) = run(listener, Api::new(runtime, router)).await {
        tracing::error!("the remote surface stopped: {e}");
    }
}

/// Serve on an already-bound listener.
pub(crate) async fn run(listener: TcpListener, api: Api) -> std::io::Result<()> {
    axum::serve(
        listener,
        app(api).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
}

pub(crate) fn app(api: Api) -> axum::Router {
    axum::Router::new()
        .merge(routes::mount())
        // Mounted outside `routes`, which carries the request timeout: an
        // event stream is meant to stay open.
        .merge(stream::mount())
        .layer(axum::middleware::from_fn(envelope::stamp))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(MAX_BODY))
        .with_state(api)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rejected_credential_is_written_down_by_its_head_or_not_at_all() {
        assert_eq!(redact(""), "<none>");
        assert_eq!(redact("hst_abcdefgh"), "<malformed>");
        let long = "hst_abcdefghijklmnopqrstuvwxyz";
        assert_eq!(redact(long), "hst_abcdefgh");
        assert!(
            long.len() > redact(long).len(),
            "the tail of a key never reaches a log"
        );
    }

    #[test]
    fn the_header_version_is_one_this_build_serves() {
        assert!(VERSIONS.contains(&CURRENT));
        assert_eq!(VERSIONS.last(), Some(&CURRENT));
    }
}
