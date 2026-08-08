//! The HTTP envelope, mirroring `hestia-web`'s `ApiResponse` so one SDK covers
//! both services. The socket envelope is deliberately not reused — HTTP already
//! has status codes for what `v` and `ok` carry
//! ([0072](../../../../docs/decisions/0072-http-is-a-second-door.md)).

use axum::http::{header, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use proto::error::ErrorInfo;
use serde::Serialize;
use serde_json::Value;

use super::status;

/// Which resource contract answered. A client that guessed the wrong `/api/vN`
/// learns why from the response it already has.
pub const API_VERSION: HeaderName = HeaderName::from_static("x-hestia-api-version");
/// Which daemon answered.
pub const DAEMON_VERSION: HeaderName = HeaderName::from_static("x-hestia-daemon");

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Body {
    success: bool,
    /// Developer-facing English, from `ErrorInfo`'s `Display`. For logs and
    /// `curl`, never for UI — a front-end localizes from `data` instead.
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'static str>,
}

/// A successful answer and the status it deserves.
pub struct Answer {
    status: StatusCode,
    message: String,
    data: Value,
    location: Option<String>,
}

impl Answer {
    pub fn new(message: impl Into<String>, data: Value) -> Answer {
        Answer {
            status: StatusCode::OK,
            message: message.into(),
            data,
            location: None,
        }
    }

    /// The work is running; its progress arrives on the event stream. 202 so a
    /// client knows to watch without inspecting the body.
    pub fn accepted(message: impl Into<String>, data: Value) -> Answer {
        Answer {
            status: StatusCode::ACCEPTED,
            ..Answer::new(message, data)
        }
    }

    /// Where the thing that was just started can be followed.
    pub fn at(mut self, location: impl Into<String>) -> Answer {
        self.location = Some(location.into());
        self
    }
}

impl IntoResponse for Answer {
    fn into_response(self) -> Response {
        let body = Json(Body {
            success: true,
            message: self.message,
            data: Some(self.data),
            code: None,
        });
        let mut response = (self.status, body).into_response();
        if let Some(value) = self.location.and_then(|l| HeaderValue::from_str(&l).ok()) {
            response.headers_mut().insert(header::LOCATION, value);
        }
        response
    }
}

/// A failure, carried whole: the coarse `code` a generic integrator switches on,
/// and the full `ErrorInfo` in `data` a Hestia-aware client localizes.
pub struct Failure(pub ErrorInfo);

impl From<ErrorInfo> for Failure {
    fn from(info: ErrorInfo) -> Failure {
        Failure(info)
    }
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let info = status::sanitize(self.0);
        let (status, code) = status::of(&info);
        let retry = match &info {
            ErrorInfo::TooManyAttempts {
                retry_after_seconds,
            } => HeaderValue::from_str(&retry_after_seconds.to_string()).ok(),
            _ => None,
        };
        let body = Json(Body {
            success: false,
            message: info.to_string(),
            data: serde_json::to_value(&info).ok(),
            code: Some(code),
        });
        let mut response = (status, body).into_response();
        if let Some(retry) = retry {
            response.headers_mut().insert(header::RETRY_AFTER, retry);
        }
        response
    }
}

/// What a route returns; the error arm is the whole failure vocabulary, so a
/// handler never chooses a status code.
pub type ApiResult = Result<Answer, Failure>;

/// Stamp both version headers on every answer, including the ones the framework
/// produced itself — a 404 for an unmounted path, a 405, a body-limit 413.
pub async fn stamp(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(API_VERSION, HeaderValue::from_static(super::CURRENT));
    if let Ok(value) = HeaderValue::from_str(common::app::VERSION) {
        headers.insert(DAEMON_VERSION, value);
    }
    response
}
