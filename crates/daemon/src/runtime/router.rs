//! Maps a channel name to a typed handler and routes a decoded request to it. An
//! unknown channel or a handler failure becomes a protocol-level error response,
//! so the caller always gets a well-formed `Response` back.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use ipc::errors;
use ipc::protocol::{Request, Response};
use proto::Contract;
use serde_json::Value;
use tracing::Instrument;

use super::HandlerContext;

/// A handler's typed failure, carrying the protocol error code to answer with.
#[derive(Debug)]
pub struct ServiceError {
    pub code: String,
    pub message: String,
}

impl ServiceError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        ServiceError {
            code: code.to_string(),
            message: message.into(),
        }
    }
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(errors::NOT_FOUND, message)
    }
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(errors::BAD_REQUEST, message)
    }
    pub fn handler_error(message: impl Into<String>) -> Self {
        Self::new(errors::HANDLER_ERROR, message)
    }
}

pub type ServiceResult<T> = Result<T, ServiceError>;

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
                    tracing::debug!("dispatch");
                    let started = std::time::Instant::now();
                    let response = handler(request, ctx).await;
                    let elapsed_ms = started.elapsed().as_millis() as u64;
                    match &response.error {
                        Some(err) => tracing::warn!(
                            code = %err.code,
                            message = %err.message,
                            elapsed_ms,
                            "request failed"
                        ),
                        None => tracing::debug!(elapsed_ms, "request ok"),
                    }
                    response
                }
                None => {
                    tracing::warn!("no handler for channel");
                    Response::failure(
                        errors::UNKNOWN_CHANNEL,
                        format!("unknown channel: {}", request.channel),
                    )
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
                        Err(e) => return Response::failure(errors::BAD_REQUEST, e.to_string()),
                    };
                    match f(params, ctx).await {
                        Ok(result) => {
                            let payload = serde_json::to_value(result).unwrap_or(Value::Null);
                            Response::success(payload)
                        }
                        Err(err) => Response::failure(err.code, err.message),
                    }
                })
            }),
        );
    }
}
