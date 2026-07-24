//! Maps a channel name to a typed handler and routes a decoded request to it. An
//! unknown channel or a handler failure becomes a protocol-level error response,
//! so the caller always gets a well-formed `Response` back.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use ipc::protocol::{Request, Response};
use proto::error::ErrorInfo;
use proto::Contract;
use serde_json::Value;
use tracing::Instrument;

use super::HandlerContext;

/// A handler's typed failure is the canonical `ErrorInfo` — the same value the
/// socket carries and a front-end localizes.
pub type ServiceResult<T> = Result<T, ErrorInfo>;

/// Map an engine `anyhow` failure to an `ErrorInfo`: if the engine raised a
/// typed `ErrorInfo` (a user-facing failure a front-end can localize), surface
/// it; otherwise fall back to `Internal`, carrying the un-localized English
/// chain in `detail`.
pub fn engine_error(e: anyhow::Error) -> ErrorInfo {
    if let Some(info) = e.downcast_ref::<ErrorInfo>() {
        return info.clone();
    }
    if let Some(reauth) = e.downcast_ref::<engine::ReauthRequired>() {
        return ErrorInfo::SessionExpired {
            reference: reauth.reference.clone(),
        };
    }
    ErrorInfo::Internal {
        detail: format!("{e:#}"),
    }
}

/// Serialize an `ErrorInfo` into an error `Response`.
pub fn error_response(info: ErrorInfo) -> Response {
    Response::failure(serde_json::to_value(&info).unwrap_or(Value::Null))
}

/// Channels locked until a Minecraft account is signed in: you cannot use
/// Minecraft you don't own.
fn requires_account(channel: &str) -> bool {
    channel.starts_with("instance.") || channel.starts_with("sync.")
}

type BoxFuture = Pin<Box<dyn Future<Output = Response> + Send>>;
type Handler = Arc<dyn Fn(Request, HandlerContext) -> BoxFuture + Send + Sync>;

#[derive(Default)]
pub struct Router {
    handlers: HashMap<String, Handler>,
}

impl Router {
    pub fn on(&mut self, channel: &str, handler: Handler) {
        self.handlers.insert(channel.to_string(), handler);
    }

    pub async fn route(&self, request: Request, ctx: HandlerContext) -> Response {
        let span = tracing::info_span!(
            "req",
            channel = %request.channel,
            id = request.id.unwrap_or_default()
        );
        async move {
            match self.handlers.get(&request.channel) {
                Some(handler) => {
                    if requires_account(&request.channel)
                        && !ctx.runtime.engine().accounts().has_account()
                    {
                        tracing::warn!("rejected: no signed-in account");
                        return error_response(ErrorInfo::SignInRequired);
                    }
                    tracing::debug!("dispatch");
                    let started = std::time::Instant::now();
                    let response = handler(request, ctx).await;
                    let elapsed_ms = started.elapsed().as_millis() as u64;
                    match &response.error {
                        Some(err) => tracing::warn!(error = %err, elapsed_ms, "request failed"),
                        None => tracing::debug!(elapsed_ms, "request ok"),
                    }
                    response
                }
                None => {
                    tracing::warn!("no handler for channel");
                    error_response(ErrorInfo::UnknownChannel {
                        channel: request.channel.clone(),
                    })
                }
            }
        }
        .instrument(span)
        .await
    }
}

/// The registrar handed to every service: binds a typed contract handler onto the
/// router. The channel name and payload shapes come from the contract, so a
/// service physically cannot drift from the client SDK.
pub struct Channels<'r> {
    router: &'r mut Router,
}

impl<'r> Channels<'r> {
    pub fn new(router: &'r mut Router) -> Self {
        Channels { router }
    }

    /// Register a handler for contract `C`: decode `C::Params` (a malformed
    /// payload answers `bad_request`), invoke `f`, and encode the returned
    /// `C::Result`. The handler returns `ServiceError` for a typed failure.
    pub fn handle<C, F, Fut>(&mut self, f: F)
    where
        C: Contract + 'static,
        F: Fn(C::Params, HandlerContext) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = ServiceResult<C::Result>> + Send + 'static,
    {
        self.router.on(
            C::CHANNEL,
            Arc::new(move |req: Request, ctx: HandlerContext| {
                let f = f.clone();
                Box::pin(async move {
                    let params: C::Params = match serde_json::from_value(req.payload) {
                        Ok(p) => p,
                        Err(e) => {
                            return error_response(ErrorInfo::MalformedRequest {
                                detail: e.to_string(),
                            })
                        }
                    };
                    match f(params, ctx).await {
                        Ok(result) => {
                            let payload = serde_json::to_value(result).unwrap_or(Value::Null);
                            Response::success(payload)
                        }
                        Err(info) => error_response(info),
                    }
                })
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::requires_account;

    #[test]
    fn gates_instance_and_sync_channels() {
        for channel in [
            "instance.launch",
            "instance.list",
            "instance.content.add",
            "instance.profile.apply",
            "sync.set",
            "instance.sync.adopt",
        ] {
            assert!(requires_account(channel), "{channel} should be gated");
        }
    }

    #[test]
    fn leaves_other_channels_open() {
        for channel in [
            "account.list",
            "server.start",
            "server.content.add",
            "content.search",
            "profile.list",
            "java.install",
        ] {
            assert!(!requires_account(channel), "{channel} should be open");
        }
    }
}
