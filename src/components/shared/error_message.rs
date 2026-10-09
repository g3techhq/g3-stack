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
        // The server withholds the message of a fault of its own (see
        // `api_errors`), leaving only a 5xx code; a refusal keeps its text.
        Some(ServerFnError::ServerError { code, .. }) if *code >= 500 => {
            SOMETHING_WENT_WRONG.to_string()
        }
        Some(ServerFnError::ServerError { message, .. }) => friendly(message),
        Some(ServerFnError::Request(_)) => {
            "Couldn't reach the server. Please try again.".to_string()
        }
        _ => friendly(&err.to_string()),
    }
}

/// The status code a server function failed with: 4xx for a refusal written
/// for the member, 5xx for a fault. A call site that has better words for one
/// of those (a sign-in that failed, an import that broke) matches on it
/// instead of showing [`error_message`].
pub fn error_code(err: &CapturedError) -> Option<u16> {
    match err.downcast_ref::<ServerFnError>() {
        Some(ServerFnError::ServerError { code, .. }) => Some(*code),
        _ => None,
    }
}

/// What a member sees when something failed in a way they can do nothing
/// about; the real error stays out of sight.
pub const SOMETHING_WENT_WRONG: &str = "Something went wrong. Please try again.";

/// `message` if it was written for a member, else [`SOMETHING_WENT_WRONG`].
///
/// Server functions return both kinds as one string: moderation and
/// validation refusals are sentences ("Tag can't contain a link."), while a
/// `?` on a database or decoding error carries its diagnostics ("Failed to
/// deserialize field ..."). A written message is a short sentence ending in
/// punctuation and free of technical vocabulary; anything else is treated as
/// a leak. Use this for every error text that reaches the screen, including
/// ones that don't come from a server function (connectors, native plugins).
pub fn friendly(message: &str) -> String {
    if is_member_facing(message) {
        message.trim().to_string()
    } else {
        SOMETHING_WENT_WRONG.to_string()
    }
}

fn is_member_facing(message: &str) -> bool {
    const TECHNICAL: [&str; 24] = [
        "deserializ",
        "serializ",
        "decode",
        "variant",
        "surreal",
        "database",
        "query",
        "sql",
        "panick",
        "tokio",
        "hyper",
        "reqwest",
        "os error",
        "server function",
        "details:",
        "field '",
        "type '",
        "stack",
        "thrown",
        "parse error",
        "no such",
        "::",
        "{",
        "}",
    ];
    let message = message.trim();
    let lower = message.to_lowercase();
    message.len() <= 240
        && message.chars().next().is_some_and(char::is_uppercase)
        && message.ends_with(['.', '!', '?'])
        && !TECHNICAL.iter().any(|word| lower.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::dioxus_fullstack::RequestError;

    #[test]
    fn server_messages_are_shown_as_written() {
        let err = CapturedError::from(ServerFnError::ServerError {
            message: "Tag can't contain a link.".to_string(),
            code: 400,
            details: None,
        });
        assert_eq!(error_message(&err), "Tag can't contain a link.");
    }

    #[test]
    fn a_suppressed_server_fault_gets_the_plain_sentence() {
        let err = CapturedError::from(ServerFnError::ServerError {
            message: String::new(),
            code: 500,
            details: None,
        });
        assert_eq!(error_message(&err), SOMETHING_WENT_WRONG);
        assert_eq!(error_code(&err), Some(500));
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
    fn written_sentences_pass_through() {
        let err = CapturedError::msg("Something odd.");
        assert_eq!(error_message(&err), "Something odd.");
        assert_eq!(
            friendly("Invalid email or password."),
            "Invalid email or password."
        );
    }

    #[test]
    fn diagnostics_are_replaced() {
        for leaked in [
            "Failed to deserialize field 'color_scheme' on type 'User': Failed to decode ColorScheme, no variants matched",
            "There was a problem with the database: connection reset.",
            "error running server function: oops (details: None)",
            "Parse error on line 3.",
            "",
        ] {
            assert_eq!(friendly(leaked), SOMETHING_WENT_WRONG, "{leaked}");
        }
    }
}
