//! Sessions and sign-in.
//!
//! Sessions come from `g3-auth`: an `axum_session` cookie backed by the
//! `sessions` table, resolved to the signed-in user on every request, and a
//! guard that denies by default. `StateExtractor` in `session.rs` is what
//! every server function actually names.
//!
//! Only guest sign-in is implemented. It is the smallest thing that produces
//! a real account and a real session, which is what the rest of the template
//! needs to demonstrate anything. See docs/adding-a-feature.md for where
//! email or OAuth providers slot in.

mod account;
mod guest;
mod session;

pub use account::*;
pub use guest::*;
#[cfg(feature = "server")]
pub use session::*;

use dioxus::prelude::*;

/// Cheap "is there a session?" probe for the splash screen.
///
/// Public: the splash asks this before it knows whether anyone is signed in.
#[g3_auth::public]
#[get("/api/v1/is_signed_in", crate::StateExtractor { auth_session, .. }: crate::StateExtractor)]
pub async fn is_signed_in() -> Result<bool> {
    Ok(auth_session.is_authenticated())
}

/// Ends the session. Unlike the auth guard, which runs on every request, this
/// is the right place to rotate it: the user asked.
#[post("/api/v1/sign_out", crate::StateExtractor { auth_session, .. }: crate::StateExtractor)]
pub async fn sign_out() -> Result<()> {
    auth_session.logout_user();
    Ok(())
}
