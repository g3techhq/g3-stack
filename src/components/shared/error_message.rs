use dioxus::CapturedError;
use dioxus::prelude::*;

/// What to show a member when a server function fails.
///
/// Dioxus's own `Display` wraps a server's message as "error running server
/// function: … (details: None)", but the messages the server writes —
/// moderation refusals especially — are already worded for members, so
/// this unwraps them. A request that never reached the server gets one
/// plain sentence instead of the transport's diagnostics.
pub fn error_message(err: &CapturedError) -> String {
    match err.downcast_ref::<ServerFnError>() {
        Some(ServerFnError::ServerError { message, .. }) => message.clone(),
        Some(ServerFnError::Request(_)) => {
            "Couldn't reach the server. Please try again.".to_string()
        }
        _ => err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::dioxus_fullstack::RequestError;

    #[test]
    fn server_messages_are_shown_as_written() {
        let err = CapturedError::from(ServerFnError::ServerError {
            message: "Tag can't contain a link.".to_string(),
            code: 500,
            details: None,
        });
        assert_eq!(error_message(&err), "Tag can't contain a link.");
    }

    #[test]
    fn network_failures_get_a_plain_sentence() {
        let err = CapturedError::from(ServerFnError::Request(RequestError::Connect(
            "connection refused".to_string(),
        )));
        assert_eq!(
            error_message(&err),
            "Couldn't reach the server. Please try again."
        );
    }

    #[test]
    fn anything_else_falls_back_to_its_display() {
        let err = CapturedError::msg("Something odd.");
        assert_eq!(error_message(&err), "Something odd.");
    }
}
