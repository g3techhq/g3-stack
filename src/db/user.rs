use super::impl_id;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use surrealdb_types::{Datetime, RecordId, SurrealValue};

/// Mirrors `database/schema/user.surql`.
///
/// `SurrealValue` is what lets this struct go straight into `db.create(..)`
/// and come straight back out of `response.take(..)`; `Serialize`/
/// `Deserialize` are what carry it over the wire to the client. A row type
/// generally wants all three.
#[derive(Serialize, Deserialize, SurrealValue, Clone, PartialEq, Debug)]
pub struct User {
    pub id: RecordId,
    pub handle: String,
    pub display_name: String,
    pub email: Option<String>,
    pub appearance_mode: AppearanceMode,
    pub color_scheme: ColorScheme,
    pub created_at: Datetime,
}

impl_id!(User);

/// The insert shape: same fields minus `id`, which the database assigns.
///
/// Kept separate from `User` rather than making `id` optional, so a function
/// holding a `User` can always name its id without unwrapping.
///
/// Only ever constructed inside a server-function body, which the `#[post]`
/// macro replaces with an HTTP call on client builds — hence the `cfg_attr`.
#[cfg_attr(not(feature = "server"), allow(dead_code))]
#[derive(Serialize, Deserialize, SurrealValue, Clone, PartialEq, Debug)]
pub struct CreateUser {
    pub handle: String,
    pub display_name: String,
    pub email: Option<String>,
    pub appearance_mode: AppearanceMode,
    pub color_scheme: ColorScheme,
    pub created_at: Datetime,
}

impl Default for CreateUser {
    fn default() -> Self {
        Self {
            handle: String::new(),
            display_name: "New User".to_string(),
            email: None,
            appearance_mode: AppearanceMode::Ios,
            color_scheme: ColorScheme::Light,
            created_at: Datetime::default(),
        }
    }
}

/// Which platform aesthetic g3-ui renders — the `mode` in Ionic's sense.
///
/// The three attribute families each answer a different question and none
/// substitutes for the others:
///
/// - `#[surreal(untagged)]` plus `#[surreal(value = "ios")]` is what makes
///   this store as the bare string `"ios"`. **Without them the derive writes
///   `{ Ios: {} }`**, which a column declared `TYPE "ios" | "md"` rejects at
///   insert time with a coercion error — and `serde`'s `rename` does not
///   affect `SurrealValue` at all.
/// - `#[strum(to_string = ..)]` gives the same spelling to `Display`.
/// - `Serialize`/`Deserialize` carry it over the wire to the client.
///
/// The `surreal` values match the literals `database/schema/user.surql`
/// constrains the column to, so the Rust type and the database agree on
/// exactly two spellings.
#[derive(
    Serialize, Deserialize, SurrealValue, Display, EnumString, Clone, Copy, PartialEq, Eq, Debug,
)]
#[surreal(untagged)]
pub enum AppearanceMode {
    #[surreal(value = "ios")]
    #[strum(to_string = "ios")]
    Ios,
    #[surreal(value = "md")]
    #[strum(to_string = "md")]
    Md,
}

/// Light or dark. Separate from `AppearanceMode` because they are genuinely
/// independent: iOS dark and Material light are both ordinary combinations.
#[derive(
    Serialize, Deserialize, SurrealValue, Display, EnumString, Clone, Copy, PartialEq, Eq, Debug,
)]
#[surreal(untagged)]
pub enum ColorScheme {
    #[surreal(value = "light")]
    #[strum(to_string = "light")]
    Light,
    #[surreal(value = "dark")]
    #[strum(to_string = "dark")]
    Dark,
}

#[cfg(test)]
mod tests {
    use super::*;
    use surrealdb_types::Value;

    /// A regression test for a failure that only ever appears at runtime.
    ///
    /// Without `#[surreal(untagged)]` and `#[surreal(value = ..)]` these
    /// derive to `{ Ios: {} }`, which compiles, type-checks, and passes every
    /// other test — then fails on the first insert with "Expected `'ios' |
    /// 'md'` but found `{ Ios: {  } }`", because the column in
    /// `database/schema/user.surql` constrains it to those two strings.
    ///
    /// Copy this test whenever you add an enum that a schema constrains.
    #[test]
    fn appearance_enums_store_as_the_bare_strings_the_schema_expects() {
        assert_eq!(
            AppearanceMode::Ios.into_value(),
            Value::String("ios".to_string())
        );
        assert_eq!(
            AppearanceMode::Md.into_value(),
            Value::String("md".to_string())
        );
        assert_eq!(
            ColorScheme::Light.into_value(),
            Value::String("light".to_string())
        );
        assert_eq!(
            ColorScheme::Dark.into_value(),
            Value::String("dark".to_string())
        );
    }

    #[test]
    fn they_come_back_out_again() {
        assert_eq!(
            AppearanceMode::from_value(Value::String("md".to_string())).expect("md is valid"),
            AppearanceMode::Md
        );
        assert_eq!(
            ColorScheme::from_value(Value::String("dark".to_string())).expect("dark is valid"),
            ColorScheme::Dark
        );
    }

    /// `Display` has to agree with the stored form, or a value interpolated
    /// into a log line, a URL, or a hand-written query says something the
    /// database would not recognize.
    #[test]
    fn display_matches_the_stored_spelling() {
        assert_eq!(AppearanceMode::Ios.to_string(), "ios");
        assert_eq!(ColorScheme::Dark.to_string(), "dark");
    }
}

/// The signed-in account, or `None` for a visitor without a session.
///
/// `#[get]` rather than `#[post]`: it is a read. `#[public]` precisely so it
/// can answer `None` instead of a `401` the splash would have to special-case.
#[g3_auth::public]
#[get("/api/v1/user", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
pub async fn get_current_user() -> Result<Option<User>> {
    if session_user.anonymous {
        return Ok(None);
    }

    Ok(db
        .query("SELECT * FROM $user")
        .bind(("user", session_user.record_id()))
        .await?
        .take::<Option<User>>(0)?)
}

/// Persists an appearance change so it follows the account to other devices.
#[post("/api/v1/update_appearance", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
pub async fn update_appearance(mode: AppearanceMode, scheme: ColorScheme) -> Result<User> {
    db.query("UPDATE $user SET appearance_mode = $mode, color_scheme = $scheme RETURN AFTER")
        // `session_user`, never an id from the request: the client can say
        // anything, and an owner field taken from the body is an account
        // takeover waiting to be typed into a fetch call.
        .bind(("user", session_user.record_id()))
        .bind(("mode", mode))
        .bind(("scheme", scheme))
        .await?
        .take::<Option<User>>(0)?
        .ok_or_else(|| dioxus::CapturedError::msg("Failed to save appearance settings."))
}
