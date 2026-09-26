//! The session guard and the extractor every server function uses.

cfg_if::cfg_if! { if #[cfg(feature = "server")] {

/// Where a signed-out page load lands: the splash, which owns `/`.
///
/// Not straight to sign-in: the server cannot tell a stale cookie from a
/// never-signed-in visitor without asking the database, and doing that inside
/// a middleware would mean a round trip on every request. So the guard sends
/// them to the splash, and the splash asks once. It is `/` because that is
/// also where a native build starts, and a native build never makes a
/// document request for this guard to redirect.
///
/// Kept in step with the route table by a test at the bottom of this file
/// rather than by a comment.
pub const SPLASH_PATH: &str = "/";

/// Must stay reachable without a session.
pub const SIGN_IN_PATH: &str = "/signin";

use std::sync::Arc;

use anyhow::{Error, Result, anyhow};
use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts, Request},
    http::{StatusCode, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_session_auth::{AuthSession, Authentication};
use surrealdb::{Surreal, engine::remote::ws::Client};
use surrealdb_types::{RecordId, ToSql};

use crate::{AppServerState, auth::SurrealSessionPool, db::User};

/// The identity `axum_session_auth` resolves a session cookie to. Kept small
/// on purpose - it is loaded on every request. Anything more than an id and a
/// display name should be a separate query from the server function that
/// needs it.
#[derive(Debug, Clone)]
pub struct SessionUser {
    pub id: String,
    pub anonymous: bool,
    pub username: String,
}

/// Written out rather than derived. `#[derive(Default)]` would produce
/// `anonymous: false` - "a signed-in user whose id happens to be empty" -
/// which is exactly backwards, since `StateExtractor` reaches for the default
/// precisely when the request carried no usable session. Every
/// `session_user.anonymous` check would then wave a signed-out visitor
/// through against an empty record id.
impl Default for SessionUser {
    fn default() -> Self {
        Self {
            id: String::new(),
            anonymous: true,
            username: String::new(),
        }
    }
}

impl SessionUser {
    /// The `user:` record id to bind into queries. This - not anything from
    /// the request body - is what an owner field should be set from.
    pub fn record_id(&self) -> RecordId {
        RecordId::new("user", self.id.clone())
    }
}

#[async_trait]
impl Authentication<SessionUser, String, Arc<Surreal<Client>>> for SessionUser {
    async fn load_user(user_id: String, db: Option<&Arc<Surreal<Client>>>) -> Result<SessionUser> {
        let user = db
            .ok_or_else(|| anyhow!("Database connection not provided"))?
            .query("SELECT * FROM $id")
            .bind(("id", RecordId::new("user", user_id)))
            .await
            .map_err(Error::from)?
            .take::<Option<User>>(0)
            .map_err(Error::from)?
            .ok_or_else(|| anyhow!("User not found"))?;

        Ok(SessionUser {
            id: user.id.key.to_sql(),
            anonymous: false,
            username: user.display_name,
        })
    }

    fn is_authenticated(&self) -> bool {
        !self.anonymous
    }

    fn is_active(&self) -> bool {
        !self.anonymous
    }

    fn is_anonymous(&self) -> bool {
        self.anonymous
    }
}

pub type AppAuthSession =
    AuthSession<SessionUser, String, SurrealSessionPool<Client>, Arc<Surreal<Client>>>;

/// What every server function in this app extracts.
///
/// Bundling the database handle, the raw session, and the resolved user into
/// one extractor is what keeps a server function's signature down to the
/// thing it actually does:
///
/// ```ignore
/// #[post("/api/v1/create_note", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
/// pub async fn create_note(title: String) -> Result<Note> { .. }
/// ```
///
/// Destructure only the fields you use and let `..` swallow the rest.
pub struct StateExtractor {
    pub db: Arc<Surreal<Client>>,
    pub auth_session: AppAuthSession,
    pub session_user: SessionUser,
}

impl<S> FromRequestParts<S> for StateExtractor
where
    AppServerState: FromRef<S>,
    S: Send + Sync,
{
    /// NOT `()`. axum renders that as an empty 200, which the server-function
    /// client reads as `null` - so a missing `AuthSessionLayer` would reach
    /// `get_current_user` as a perfectly ordinary `Ok(None)`, i.e. "nobody is
    /// signed in", and quietly bounce a signed-in user to the sign-in screen.
    /// A misconfigured layer stack has to look like the server fault it is.
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        let db = AppServerState::from_ref(state).db;
        let auth_session = parts
            .extensions
            .get::<AppAuthSession>()
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Auth session missing. Is `AuthSessionLayer` installed?",
            ))?
            .clone();
        let session_user = auth_session.current_user.clone().unwrap_or_default();

        Ok(StateExtractor {
            db,
            auth_session,
            session_user,
        })
    }
}

/// Paths that must answer a visitor with no session.
///
/// Add to this list whenever you add a route or endpoint that has to work
/// before sign-in - a legal page an app store links to, an OAuth callback, a
/// public share link. Forgetting one shows up as a redirect loop, or as a
/// screen that can never load.
fn is_unsecured_path(path: &str) -> bool {
    matches!(path, SPLASH_PATH | SIGN_IN_PATH)
        || matches!(
            path,
            // These have to give the splash a straight answer rather than a
            // redirect it cannot decode.
            "/api/v1/user" | "/api/v1/is_signed_in" | "/api/v1/sign_in_guest"
        )
        || path.starts_with("/wasm/")
        || path.starts_with("/assets/")
        // The browser requests this on a signed-out page load without asking
        // the app first. Guarding it answers 401 and allocates a session row
        // per request, which shows up as noise in the log and a missing tab
        // icon on the sign-in screen.
        || path == "/favicon.ico"
}

/// Whether this request is a browser navigation, as opposed to a `fetch`.
///
/// Only a navigation can act on a redirect. `fetch` follows one
/// transparently, so a server function redirected to the sign-in page gets
/// that page's HTML back under a 200 where it expected its own JSON - which
/// reaches the app as an opaque deserialization error indistinguishable from
/// a real failure. That is what strands a client when the backend restarts
/// and the session named by the cookie no longer resolves: every screen's
/// fetch quietly fails and nothing sends the user back to sign in.
fn is_document_navigation(request: &Request) -> bool {
    // Every browser supporting Fetch Metadata sends `document` on a real
    // navigation and `empty` on a `fetch`, which settles the question on its
    // own for the case that actually matters.
    if let Some(destination) = request
        .headers()
        .get("sec-fetch-dest")
        .and_then(|value| value.to_str().ok())
    {
        return destination == "document";
    }

    // Without it, go by what was asked for: every server function lives under
    // `/api/`, and everything else is a page. A client that sends no Fetch
    // Metadata at all - curl, a probe, an old browser - keeps getting the
    // redirect it always did.
    !request.uri().path().starts_with("/api/")
}

/// Shaped like dioxus-fullstack's own error payload so the client decodes it
/// into a `ServerFnError::ServerError` carrying `code: 401`, rather than an
/// opaque decode failure. A caller that wants to react to an expired session
/// then has something unambiguous to match on.
const UNAUTHORIZED_BODY: &str =
    r#"{"message":"Your session has expired. Please sign in again.","code":401}"#;

/// Rejects anything that needs a session and does not have one.
///
/// Installed as a layer in `main`, so an endpoint is protected by default:
/// adding a server function without thinking about auth gets you the safe
/// behavior, and opening one up is a deliberate edit to `is_unsecured_path`.
pub async fn auth_check(request: Request, next: Next) -> Response {
    if !is_unsecured_path(request.uri().path()) {
        let is_authenticated = request
            .extensions()
            .get::<AppAuthSession>()
            .is_some_and(AppAuthSession::is_authenticated);

        if !is_authenticated {
            // Deliberately no `logout_user()` here. This guard runs on every
            // request, and rotating the session from a read path lets one
            // unlucky request invalidate a session that other in-flight
            // requests are still using. Signing in issues a fresh session
            // anyway, and `sign_out` clears the old one where that belongs.
            if is_document_navigation(&request) {
                return Redirect::to(SPLASH_PATH).into_response();
            }

            return (
                StatusCode::UNAUTHORIZED,
                [(header::CONTENT_TYPE, "application/json")],
                UNAUTHORIZED_BODY,
            )
                .into_response();
        }
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    fn request_with(path: &str, headers: &[(&str, &str)]) -> Request {
        let mut builder = Request::builder().uri(path);
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        builder.body(Body::empty()).expect("valid test request")
    }

    #[test]
    fn browser_navigations_are_documents() {
        assert!(is_document_navigation(&request_with(
            "/notes",
            &[
                ("sec-fetch-dest", "document"),
                ("accept", "text/html,application/xhtml+xml"),
            ]
        )));
    }

    #[test]
    fn server_function_calls_are_not_documents() {
        // Fetch Metadata settles it even when the other headers would read as
        // a navigation - the case that matters, since a redirect served to a
        // `fetch` comes back as sign-in HTML the caller cannot decode.
        assert!(!is_document_navigation(&request_with(
            "/api/v1/notes",
            &[("sec-fetch-dest", "empty"), ("accept", "text/html")]
        )));
    }

    #[test]
    fn falls_back_to_the_path_without_fetch_metadata() {
        assert!(!is_document_navigation(&request_with("/api/v1/notes", &[])));
        assert!(is_document_navigation(&request_with("/", &[])));
    }

    #[test]
    fn the_splash_and_the_questions_it_asks_stay_reachable_signed_out() {
        // The whole point of redirecting to the splash is that it can always
        // render and always get an answer. Guarding either would send it back
        // to itself.
        assert!(is_unsecured_path(SPLASH_PATH));
        assert!(is_unsecured_path(SIGN_IN_PATH));
        assert!(is_unsecured_path("/api/v1/is_signed_in"));
        assert!(is_unsecured_path("/api/v1/user"));
    }

    #[test]
    fn application_endpoints_are_guarded_by_default() {
        assert!(!is_unsecured_path("/api/v1/notes"));
        assert!(!is_unsecured_path("/api/v1/create_note"));
        assert!(!is_unsecured_path("/notes"));
    }

    /// The two path constants above are the guard's copy of what the router
    /// believes. A route renamed in `app.rs` without updating them here is a
    /// redirect to a 404, which is exactly the kind of thing a comment saying
    /// "keep these in sync" does not catch.
    #[test]
    fn the_unsecured_paths_match_the_route_table() {
        use crate::app::Route;

        assert_eq!(Route::Splash {}.to_string(), SPLASH_PATH);
        assert_eq!(Route::SignIn {}.to_string(), SIGN_IN_PATH);
    }

    #[test]
    fn the_client_bundle_loads_before_there_is_a_session() {
        // The WASM and its assets are fetched by the signed-out splash page
        // itself; guarding them would leave nothing to run.
        assert!(is_unsecured_path("/wasm/g3-app_bg.wasm"));
        assert!(is_unsecured_path("/assets/tailwind.css"));
        assert!(is_unsecured_path("/favicon.ico"));
    }
}

}}
