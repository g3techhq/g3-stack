//! Where a native client sends its server-function calls.
//!
//! Only mobile builds set anything. A web client is served by the same server
//! it calls, so Dioxus's default — the origin that served the page — is already
//! right, and baking a URL into the WASM would only make the same bundle wrong
//! on every other hostname. A phone has no page origin: its server has to be
//! compiled into the signed app.

#[cfg_attr(not(feature = "mobile"), allow(dead_code))]
pub fn configure() {
    #[cfg(feature = "mobile")]
    if let Some(url) = mobile_server_url() {
        dioxus::fullstack::set_server_url(url);
    }
}

#[cfg(feature = "mobile")]
fn mobile_server_url() -> Option<&'static str> {
    let Some(configured) = option_env!("SERVER_URL") else {
        if cfg!(debug_assertions) {
            // Unset in development lets `dx serve --platform android|ios` point
            // the app at the dev server it started, through
            // DIOXUS_DEVSERVER_IP and DIOXUS_DEVSERVER_PORT.
            return None;
        }
        panic!("SERVER_URL must be set when building a release mobile app");
    };

    let configured = configured.strip_suffix('/').unwrap_or(configured);
    validate(configured, cfg!(debug_assertions))
        .unwrap_or_else(|error| panic!("Invalid SERVER_URL for a mobile build: {error}"));
    Some(configured)
}

/// A release app talks to its server over HTTPS, or the session cookie crosses
/// the network in the clear. Loopback HTTP is allowed only in debug builds.
#[cfg(any(feature = "mobile", test))]
fn validate(value: &str, allow_loopback_http: bool) -> Result<(), String> {
    let parsed = url::Url::parse(value).map_err(|error| format!("not a valid URL: {error}"))?;
    let loopback = matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));

    if parsed.scheme() != "https" && !(allow_loopback_http && loopback && parsed.scheme() == "http")
    {
        return Err("must use HTTPS; debug builds may use HTTP on localhost".into());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("must not contain credentials".into());
    }
    // Server functions are mounted at `/api/..` on the origin. A path here
    // would be prepended to every call and miss them all.
    if parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() {
        return Err("must be a bare origin without a path, query, or fragment".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate;

    #[test]
    fn accepts_https_origins() {
        assert!(validate("https://app.example.com", false).is_ok());
        assert!(validate("https://app.example.com:8443", false).is_ok());
    }

    #[test]
    fn permits_loopback_http_only_in_debug() {
        assert!(validate("http://localhost:8080", true).is_ok());
        assert!(validate("http://localhost:8080", false).is_err());
        assert!(validate("http://192.168.1.20:8080", true).is_err());
    }

    #[test]
    fn rejects_anything_that_is_not_a_bare_origin() {
        for value in [
            "https://user:pass@example.com",
            "https://example.com/api",
            "https://example.com?env=staging",
            "https://example.com#fragment",
            "example.com",
        ] {
            assert!(
                validate(value, false).is_err(),
                "{value} should be rejected"
            );
        }
    }
}
