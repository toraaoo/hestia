//! Who is calling, and whether they may — two questions, answered separately.
//! The extractor answers the first; every route answers the second by naming the
//! scope it costs. A valid key is never permission for whatever route it reached,
//! which is what Wings' CVE-2026-54593 got wrong
//! ([0075](../../../../docs/decisions/0075-the-remote-surface-is-an-allowlist.md)).

use std::net::{IpAddr, SocketAddr};

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use engine::Grant;
use proto::error::ErrorInfo;
use proto::remote::Scope;

use super::envelope::Failure;
use super::Api;

/// A request that carried a key this node recognises.
pub struct Key(pub Grant);

impl Key {
    /// The one line a route spends to be safe. Yields the grant, so a handler
    /// needing the narrowing too reaches it without a second lookup.
    pub fn require(&self, scope: Scope) -> Result<&Grant, Failure> {
        if !self.0.holds(scope) {
            tracing::warn!(key = %self.0.prefix, %scope, "rejected: key does not hold the scope");
            return Err(ErrorInfo::RemoteScopeRequired { scope }.into());
        }
        Ok(&self.0)
    }
}

impl FromRequestParts<Api> for Key {
    type Rejection = Failure;

    async fn from_request_parts(parts: &mut Parts, api: &Api) -> Result<Key, Failure> {
        let caller = caller(parts, api.trusts_proxy);
        if let Some(seconds) = api.throttle.blocked_for(caller) {
            tracing::warn!(%caller, seconds, "rejected: too many rejected keys");
            return Err(ErrorInfo::TooManyAttempts {
                retry_after_seconds: seconds,
            }
            .into());
        }

        let presented = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .unwrap_or_default()
            .trim();

        match api.runtime.engine().remote().verify(presented) {
            Some(grant) => {
                tracing::debug!(key = %grant.prefix, %caller, "request authenticated");
                Ok(Key(grant))
            }
            None => {
                api.throttle.rejected(caller);
                // By prefix only: a rejected key is still a key.
                tracing::warn!(
                    key = %super::redact(presented),
                    %caller,
                    "rejected: no key on this node matches"
                );
                Err(ErrorInfo::RemoteKeyRejected.into())
            }
        }
    }
}

/// Who to hold the rate limit against. `X-Forwarded-For` is believed only when
/// `remote.trusted-proxy` says a proxy we control sets it — a client can
/// otherwise spend somebody else's budget. Left-most entry is the client.
fn caller(parts: &Parts, trusts_proxy: bool) -> IpAddr {
    let socket = parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(address)| address.ip())
        .unwrap_or(IpAddr::from([0, 0, 0, 0]));
    if !trusts_proxy {
        return socket;
    }
    parts
        .headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .and_then(|first| first.trim().parse().ok())
        .unwrap_or(socket)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use proto::remote::Scope;

    fn grant(scopes: Vec<Scope>) -> Key {
        Key(Grant {
            id: "k1".into(),
            prefix: "hst_abcdefgh".into(),
            scopes,
            servers: Vec::new(),
        })
    }

    fn parts(forwarded: Option<&str>, peer: [u8; 4]) -> Parts {
        let mut request = Request::builder();
        if let Some(forwarded) = forwarded {
            request = request.header("x-forwarded-for", forwarded);
        }
        let request = request.body(()).unwrap();
        let (mut parts, ()) = request.into_parts();
        parts
            .extensions
            .insert(ConnectInfo(SocketAddr::from((peer, 40000))));
        parts
    }

    #[test]
    fn a_route_gets_the_scope_it_names_and_nothing_else() {
        let key = grant(vec![Scope::ServerRead]);
        assert!(key.require(Scope::ServerRead).is_ok());
        for withheld in [
            Scope::ServerControl,
            Scope::ServerWrite,
            Scope::ServerBackup,
            Scope::ServerCreate,
            Scope::ServerDelete,
        ] {
            assert!(
                key.require(withheld).is_err(),
                "a read key must not reach {withheld}"
            );
        }
    }

    #[test]
    fn a_refusal_names_the_scope_that_was_missing() {
        let refused = grant(vec![Scope::ServerRead])
            .require(Scope::ServerDelete)
            .expect_err("refused");
        assert!(matches!(
            refused.0,
            ErrorInfo::RemoteScopeRequired {
                scope: Scope::ServerDelete
            }
        ));
    }

    #[test]
    fn a_forwarded_address_is_ignored_until_a_proxy_is_trusted() {
        let claimed = parts(Some("203.0.113.9"), [127, 0, 0, 1]);
        assert_eq!(caller(&claimed, false), IpAddr::from([127, 0, 0, 1]));
        assert_eq!(caller(&claimed, true), IpAddr::from([203, 0, 113, 9]));
    }

    #[test]
    fn a_trusted_proxys_chain_resolves_to_the_original_client() {
        let chained = parts(Some("203.0.113.9, 10.0.0.2, 10.0.0.3"), [10, 0, 0, 3]);
        assert_eq!(caller(&chained, true), IpAddr::from([203, 0, 113, 9]));
    }

    #[test]
    fn a_forwarded_header_that_is_not_an_address_falls_back_to_the_socket() {
        let junk = parts(Some("not-an-address"), [10, 0, 0, 3]);
        assert_eq!(caller(&junk, true), IpAddr::from([10, 0, 0, 3]));
        let absent = parts(None, [10, 0, 0, 3]);
        assert_eq!(caller(&absent, true), IpAddr::from([10, 0, 0, 3]));
    }
}
