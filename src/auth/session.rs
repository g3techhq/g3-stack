//! Who is signed in, and what a signed-out visitor may reach.
//!
//! The machinery lives in [`g3_auth`]; this names it for the app. Every
//! request needs a signed-in user unless it is for:
//!
//! - a static asset or `/.well-known/` file,
//! - a page marked `#[public]` on `Route` (see `app.rs`), or
//! - a server function marked `#[g3_auth::public]`.
//!
//! Forgetting `#[public]` is the safe mistake: a signed-out caller gets a
//! `401` the client can act on, and a signed-out page load is sent to the
//! splash. Opening something up is one line beside it, and the tests below
//! pin both lists, so it always shows up as a reviewed change.

cfg_if::cfg_if! { if #[cfg(feature = "server")] {

use surrealdb::engine::remote::ws::Client;

/// Accounts are `user` records named by `display_name`, g3-auth's defaults.
/// Deleting an account removes its row outright; if yours keeps a row
/// around instead, set `FILTER` (for example `"deleted_at = NONE"`) so a
/// session still live on another device can't load it.
pub enum AppUser {}

impl g3_auth::AuthUser for AppUser {}

/// What every server function names: the database, the auth session (to
/// sign in or out), and the signed-in user.
///
/// ```ignore
/// #[post("/api/v1/create_note", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
/// pub async fn create_note(text: String) -> Result<Note> {
///     // `session_user.record_id()` is the owner; never take it from the body.
/// }
/// ```
pub type StateExtractor = g3_auth::SessionContext<AppUser, Client>;

#[cfg(test)]
mod tests {
    use g3_auth::{AuthGuard, PublicRoutes};

    use crate::app::Route;

    /// Opening a server function to signed-out callers should always show
    /// up as a reviewed change to this list.
    #[test]
    fn the_public_endpoints_are_exactly_these() {
        assert_eq!(
            g3_auth::public_endpoints(),
            [
                "/api/v1/is_signed_in",
                "/api/v1/sign_in_guest",
                "/api/v1/user",
            ]
        );
    }

    /// The same for pages.
    #[test]
    fn the_public_pages_are_exactly_these() {
        assert_eq!(Route::PUBLIC_PATTERNS, ["/", "/signin"]);
    }

    #[test]
    fn the_guard_sends_signed_out_visitors_to_the_splash() {
        // Not straight to sign-in: the server can't tell a stale cookie from
        // a never-signed-in visitor without asking, and the splash is what
        // asks. `for_routes` also refuses a splash that isn't public.
        let guard = AuthGuard::for_routes(Route::Splash {});
        assert_eq!(guard.splash(), "/");

        assert!(guard.allows_signed_out("/signin"));
        assert!(guard.allows_signed_out("/api/v1/is_signed_in"));
        // The client bundle loads before there is a session.
        assert!(guard.allows_signed_out("/wasm/g3-app_bg.wasm"));
        assert!(guard.allows_signed_out("/assets/tailwind.css"));
        assert!(guard.allows_signed_out("/favicon.ico"));

        assert!(!guard.allows_signed_out("/notes"));
        assert!(!guard.allows_signed_out("/api/v1/notes"));
        assert!(!guard.allows_signed_out("/api/v1/create_note"));
        // The catch-all redirect sends unknown paths to the splash, but that
        // doesn't make them public.
        assert!(!guard.allows_signed_out("/no/such/page"));
    }
}

}}
