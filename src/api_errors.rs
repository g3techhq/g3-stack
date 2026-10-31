//! What a server function's failure looks like to the client.
//!
//! A server function can fail two ways. [`user_error`] is a refusal written
//! for the member ("Tag can't contain a link.") and goes out as a 400 with its
//! text. Any other error is a `?` on something the server hit (a database
//! decode, a provider timeout) and carries diagnostics a member should never
//! read, so [`suppress_server_errors`] records a fixed event and sends a code with no
//! detail. The client then words the failure itself, where it knows what the
//! member was doing (see `components::shared::error_message`).

#![cfg_attr(
    not(feature = "server"),
    allow(
        dead_code,
        reason = "Shared definitions are used by selected platform, server or test builds."
    )
)]

#[cfg(feature = "server")]
use axum::{
    body::{Body, to_bytes},
    extract::Request,
    http::header::{CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, ETAG},
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
    let response = next.run(request).await;
    if !is_api || !response.status().is_server_error() {
        return response;
    }
    let (mut parts, body) = response.into_parts();
    let bytes = to_bytes(body, MAX_ERROR_BODY).await.ok();
    let body = sanitized_failure(bytes.as_deref());
    // Never reuse validators or an encoding from the discarded error body.
    parts.headers.remove(CONTENT_ENCODING);
    parts.headers.remove(ETAG);
    parts
        .headers
        .insert(CONTENT_TYPE, "application/json".parse().unwrap());
    parts
        .headers
        .insert(CACHE_CONTROL, "private, no-store".parse().unwrap());
    parts.headers.insert(CONTENT_LENGTH, body.len().into());
    Response::from_parts(parts, Body::from(body))
}

#[cfg(feature = "server")]
fn sanitized_failure(bytes: Option<&[u8]>) -> String {
    let payload = bytes.and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok());
    // Dioxus transports an intentional user refusal as HTTP 500 with its
    // original 4xx code in the body. Keep only that written message. Malformed,
    // oversized, plain-text and arbitrary-field errors are faults too.
    if let Some(payload) = payload.as_ref()
        && let Some(code) = payload.get("code").and_then(serde_json::Value::as_u64)
        && (400..500).contains(&code)
        && let Some(message) = payload.get("message").and_then(serde_json::Value::as_str)
    {
        return serde_json::json!({"message": message, "code": code}).to_string();
    }
    eprintln!("event=api_failure");
    serde_json::json!({"message": SUPPRESSED, "code": 500}).to_string()
}

/// Records an input rejection without formatting its parser cause. Parsers and
/// upstream errors can embed uploaded contents, credentials or account details.
pub fn user_error_logged(message: &str, _cause: &dyn std::fmt::Display) -> CapturedError {
    eprintln!("event=input_rejected");
    user_error(message)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn faults_never_copy_diagnostics_or_unexpected_fields() {
        for bytes in [
            None,
            Some(b"raw-secret-canary".as_slice()),
            Some(br#"{"message":"secret-canary","code":500,"stack":"private-canary"}"#.as_slice()),
            Some(br#"{"message":"secret-canary","code":200}"#.as_slice()),
            Some(br#"{"message":"secret-canary","code":999999}"#.as_slice()),
        ] {
            let body: serde_json::Value = serde_json::from_str(&sanitized_failure(bytes)).unwrap();
            assert_eq!(body, serde_json::json!({"message":"", "code":500}));
        }
        let body = sanitized_failure(Some(
            br#"{"message":"Please choose a valid file.","code":400,"stack":"private-canary"}"#,
        ));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&body).unwrap(),
            serde_json::json!({"message":"Please choose a valid file.","code":400})
        );
    }

    #[test]
    fn parser_rejections_do_not_even_format_the_raw_cause() {
        struct PrivateCause;

        impl std::fmt::Display for PrivateCause {
            fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                panic!("a raw private cause must not be formatted");
            }
        }
        let _ = user_error_logged("Please choose a valid file.", &PrivateCause);
    }

    #[tokio::test]
    async fn middleware_suppresses_plain_text_malformed_and_oversized_faults() {
        use axum::{Router, middleware::from_fn, routing::get};
        let app = Router::new()
            .route(
                "/api/v1/{kind}",
                get(
                    |axum::extract::Path(kind): axum::extract::Path<String>| async move {
                        let body = match kind.as_str() {
                            "oversized" => "secret-canary".repeat(MAX_ERROR_BODY),
                            "malformed" => "{secret-canary".into(),
                            _ => "secret-canary".into(),
                        };
                        (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            [
                                (CONTENT_TYPE, "text/plain"),
                                (ETAG, "private-validator-canary"),
                            ],
                            body,
                        )
                    },
                ),
            )
            .layer(from_fn(suppress_server_errors));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        for kind in ["plain", "malformed", "oversized"] {
            // Use the runtime already shared by all apps; the template does
            // not need an outbound HTTP dependency just for this fixture.
            let response = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
                stream.write_all(format!("GET /api/v1/{kind}?private=query-canary HTTP/1.1\r\nHost: fixture.local\r\nConnection: close\r\nAuthorization: Bearer token-canary\r\nCookie: session=cookie-canary\r\n\r\n").as_bytes()).await.unwrap();
                let mut response = Vec::new();
                stream.read_to_end(&mut response).await.unwrap();
                String::from_utf8(response).unwrap()
            }).await.unwrap();
            let (head, body) = response.split_once("\r\n\r\n").unwrap();
            assert!(head.starts_with("HTTP/1.1 500 "));
            let headers = head
                .lines()
                .skip(1)
                .filter_map(|line| line.split_once(':'))
                .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
                .collect::<std::collections::BTreeMap<_, _>>();
            assert_eq!(headers["content-type"], "application/json");
            assert_eq!(headers["cache-control"], "private, no-store");
            assert!(!headers.contains_key("etag"));
            assert_eq!(
                headers["content-length"].parse::<usize>().unwrap(),
                body.len()
            );
            assert!(!response.contains("canary"));
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(body).unwrap(),
                serde_json::json!({"message":"", "code":500})
            );
        }
        task.abort();
    }
}
