//! Sessions and sign-in.
//!
//! The stack here is `axum_session` (a session cookie backed by a `sessions`
//! table) plus `axum_session_auth` (which resolves that session to a
//! `SessionUser`). `StateExtractor` in `session.rs` is what every server
//! function actually names.
//!
//! Only guest sign-in is implemented. It is the smallest thing that produces
//! a real account and a real session, which is what the rest of the template
//! needs to demonstrate anything. See docs/adding-a-feature.md for where
//! email or OAuth providers slot in.

mod account;
mod guest;
mod session;
mod session_store;

pub use account::*;
pub use guest::*;
#[cfg(feature = "server")]
pub use session::*;
#[cfg(feature = "server")]
pub use session_store::*;

use dioxus::prelude::*;

/// Cheap "is there a session?" probe for the splash screen.
#[get("/api/v1/is_signed_in", crate::StateExtractor { auth_session, .. }: crate::StateExtractor)]
pub async fn is_signed_in() -> Result<bool> {
    Ok(auth_session.is_authenticated())
}

/// Ends the session. Unlike the guard in `auth_check`, this is the right
/// place to rotate it: the user asked.
#[post("/api/v1/sign_out", crate::StateExtractor { auth_session, .. }: crate::StateExtractor)]
pub async fn sign_out() -> Result<()> {
    auth_session.logout_user();
    Ok(())
}
