//! What a server function's failure looks like to the client.
//!
//! A server function can fail two ways. [`user_error`] is a refusal written
//! for the member ("Tag can't contain a link.") and goes out as a 400 with its
//! text. Any other error is a `?` on something the server hit (a database
//! decode, a provider timeout) and carries diagnostics a member should never
//! read, so [`suppress_server_errors`] logs it here and sends a code with no
//! detail. The client then words the failure itself, where it knows what the
//! member was doing (see `components::shared::error_message`).

#![cfg_attr(not(feature = "server"), allow(dead_code))]

#[cfg(feature = "server")]
use axum::{
    body::{Body, to_bytes},
    extract::Request,
    http::header::{CONTENT_LENGTH, CONTENT_TYPE},
    middleware::Next,
    response::Response,
};
use dioxus::fullstack::http::StatusCode;
use dioxus::{CapturedError, fullstack::HttpError};

/// A refusal whose text is for the member, as a client error.
pub fn user_error(message: impl std::fmt::Display) -> CapturedError {
    CapturedError::from(HttpError::new(StatusCode::BAD_REQUEST, message.to_string()))
}

/// The message sent in place of a server fault's own. Empty: the client picks
/// the words, and an empty field cannot be shown by mistake.
#[cfg(feature = "server")]
const SUPPRESSED: &str = "";

/// Largest error body worth inspecting; real ones are a line.
#[cfg(feature = "server")]
const MAX_ERROR_BODY: usize = 64 * 1024;

#[cfg(feature = "server")]
pub async fn suppress_server_errors(request: Request, next: Next) -> Response {
    let is_api = request.uri().path().starts_with("/api/");
    let path = request.uri().path().to_string();
    let response = next.run(request).await;
    if !is_api || !response.status().is_server_error() {
        return response;
    }
    let is_json = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"));
    if !is_json {
        return response;
    }

    let (mut parts, body) = response.into_parts();
    let Ok(bytes) = to_bytes(body, MAX_ERROR_BODY).await else {
        return Response::from_parts(parts, Body::empty());
    };
    let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Response::from_parts(parts, Body::from(bytes));
    };

    // `code` is the error's own status; the HTTP status is 500 for every
    // failure. A refusal (< 500) keeps its message.
    let code = payload
        .get("code")
        .and_then(|code| code.as_u64())
        .unwrap_or(500);
    if code < 500 {
        return Response::from_parts(parts, Body::from(bytes));
    }
    if let Some(message) = payload.get("message").and_then(|message| message.as_str()) {
        eprintln!("[{code}] {path}: {message}");
    }
    // Rebuilt rather than edited: some failures (a request that would not
    // decode) put their diagnostics in fields of their own.
    let payload = serde_json::json!({ "message": SUPPRESSED, "code": code });
    let body = payload.to_string();
    parts.headers.insert(CONTENT_LENGTH, body.len().into());
    Response::from_parts(parts, Body::from(body))
}

/// [`user_error`] that keeps the cause out of the message and in the server
/// log, for a failure that is the member's to fix (a bad file) but whose
/// parser's own wording would only confuse.
pub fn user_error_logged(message: &str, cause: &dyn std::fmt::Display) -> CapturedError {
    eprintln!("{message} {cause}");
    user_error(message)
}
