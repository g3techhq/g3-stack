use crate::db::User;
use dioxus::prelude::*;

/// A handle nobody has to choose.
///
/// Nanosecond-derived rather than random so this needs no RNG dependency;
/// a collision is caught by the unique index on `user.handle` and surfaces as
/// a failed create rather than as two accounts sharing a name.
///
/// Server-only, like everything else the body of `sign_in_guest` touches: the
/// `#[post]` macro replaces that body with an HTTP call on client builds, so
/// anything it uses would otherwise be dead code there.
#[cfg(feature = "server")]
fn guest_handle() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();

    format!("guest{}", nanos % 100_000_000)
}

/// Creates a throwaway account and signs into it.
///
/// A real account from the first tap, so every downstream feature can assume
/// `session_user` exists instead of carrying a signed-out branch. Attaching
/// an email or an OAuth provider later fills in this same row rather than
/// creating a second one.
#[post("/api/v1/sign_in_guest", crate::StateExtractor { db, auth_session, .. }: crate::StateExtractor)]
pub async fn sign_in_guest() -> Result<User> {
    use crate::db::CreateUser;
    use surrealdb_types::ToSql;

    let handle = guest_handle();
    let user: User = db
        .create("user")
        .content(CreateUser {
            display_name: format!("Guest {}", &handle[5..]),
            handle,
            ..CreateUser::default()
        })
        .await?
        .ok_or_else(|| dioxus::CapturedError::msg("Failed to create a guest account."))?;

    auth_session.login_user(user.id.key.to_sql());
    auth_session.remember_user(true);

    Ok(user)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn a_generated_handle_is_long_enough_to_slice_for_a_display_name() {
        // `sign_in_guest` builds the display name with `&handle[5..]`, which
        // panics on a shorter string.
        let handle = guest_handle();
        assert!(handle.starts_with("guest"));
        assert!(handle.len() > 5);
    }
}
