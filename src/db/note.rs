//! The worked example: one table, wired all the way from SurrealQL to the
//! screen. Read this file alongside `database/schema/note.surql` and
//! `src/components/notes/` to see the whole path a feature takes.
//!
//! Delete all three when you start on your own domain.

use super::impl_id;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use surrealdb_types::{Datetime, RecordId, SurrealValue};

/// Mirrors `database/schema/note.surql`.
#[derive(Serialize, Deserialize, SurrealValue, Clone, PartialEq, Debug)]
pub struct Note {
    pub id: RecordId,
    pub owner: RecordId,
    pub title: String,
    pub body: String,
    pub pinned: bool,
    pub created_at: Datetime,
    pub updated_at: Datetime,
}

impl_id!(Note);

/// Insert shape. `owner` is filled in on the server from the session, never
/// from the request body — see `create_note`.
/// Only ever constructed inside a server-function body, which the `#[post]`
/// macro replaces with an HTTP call on client builds - hence the `cfg_attr`.
#[cfg_attr(not(feature = "server"), allow(dead_code))]
#[derive(Serialize, Deserialize, SurrealValue, Clone, PartialEq, Debug)]
pub struct CreateNote {
    pub owner: RecordId,
    pub title: String,
    pub body: String,
    pub pinned: bool,
    pub created_at: Datetime,
    pub updated_at: Datetime,
}

pub const TITLE_MAX: usize = 120;
pub const BODY_MAX: usize = 10_000;

/// Trims and length-checks user input.
///
/// Deliberately a plain function rather than something inside a server
/// function: it is also compiled into the client, so the form can show the
/// same message before making a request. The server still calls it — client
/// validation is a courtesy, not a control.
pub fn validate_note(title: &str, body: &str) -> Result<(String, String), String> {
    let title = title.trim();
    let body = body.trim();

    if title.is_empty() {
        return Err("A note needs a title.".to_string());
    }
    if title.chars().count() > TITLE_MAX {
        return Err(format!("Titles are limited to {TITLE_MAX} characters."));
    }
    if body.chars().count() > BODY_MAX {
        return Err(format!("Notes are limited to {BODY_MAX} characters."));
    }

    Ok((title.to_string(), body.to_string()))
}

/// Every note belonging to the signed-in user, pinned ones first.
#[get("/api/v1/notes", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
pub async fn list_notes() -> Result<Vec<Note>> {
    Ok(db
        .query("SELECT * FROM note WHERE owner = $user ORDER BY pinned DESC, updated_at DESC")
        .bind(("user", session_user.record_id()))
        .await?
        .take::<Vec<Note>>(0)?)
}

/// One note, or `None`.
///
/// The `owner = $user` clause is the authorization check. Without it, anyone
/// who can guess a record id can read anyone's note — and the id is right
/// there in the URL. Answering `None` rather than "forbidden" also avoids
/// confirming that the id exists at all.
#[post("/api/v1/get_note", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
pub async fn get_note(id: String) -> Result<Option<Note>> {
    Ok(db
        .query("SELECT * FROM $note WHERE owner = $user")
        .bind(("note", RecordId::new("note", id)))
        .bind(("user", session_user.record_id()))
        .await?
        .take::<Option<Note>>(0)?)
}

#[post("/api/v1/create_note", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
pub async fn create_note(title: String, body: String) -> Result<Note> {
    let (title, body) = validate_note(&title, &body).map_err(dioxus::CapturedError::msg)?;

    db.create("note")
        .content(CreateNote {
            owner: session_user.record_id(),
            title,
            body,
            pinned: false,
            created_at: Datetime::default(),
            updated_at: Datetime::default(),
        })
        .await?
        .ok_or_else(|| dioxus::CapturedError::msg("Failed to save the note."))
}

#[post("/api/v1/update_note", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
pub async fn update_note(id: String, title: String, body: String) -> Result<Note> {
    let (title, body) = validate_note(&title, &body).map_err(dioxus::CapturedError::msg)?;

    db.query(
        "UPDATE $note SET title = $title, body = $body, updated_at = time::now()
         WHERE owner = $user RETURN AFTER",
    )
    .bind(("note", RecordId::new("note", id)))
    .bind(("user", session_user.record_id()))
    .bind(("title", title))
    .bind(("body", body))
    .await?
    .take::<Option<Note>>(0)?
    .ok_or_else(|| dioxus::CapturedError::msg("That note no longer exists."))
}

#[post("/api/v1/set_note_pinned", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
pub async fn set_note_pinned(id: String, pinned: bool) -> Result<Note> {
    db.query("UPDATE $note SET pinned = $pinned WHERE owner = $user RETURN AFTER")
        .bind(("note", RecordId::new("note", id)))
        .bind(("user", session_user.record_id()))
        .bind(("pinned", pinned))
        .await?
        .take::<Option<Note>>(0)?
        .ok_or_else(|| dioxus::CapturedError::msg("That note no longer exists."))
}

#[post("/api/v1/delete_note", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
pub async fn delete_note(id: String) -> Result<()> {
    db.query("DELETE $note WHERE owner = $user")
        .bind(("note", RecordId::new("note", id)))
        .bind(("user", session_user.record_id()))
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_is_required() {
        assert!(validate_note("", "body").is_err());
        assert!(validate_note("   ", "body").is_err());
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_rather_than_stored() {
        let (title, body) = validate_note("  Groceries  ", "  milk\n").expect("valid");
        assert_eq!(title, "Groceries");
        assert_eq!(body, "milk");
    }

    #[test]
    fn lengths_are_counted_in_characters_not_bytes() {
        // A limit measured in bytes would reject a title of emoji or
        // non-Latin script well short of the count the user was shown.
        let title = "é".repeat(TITLE_MAX);
        assert!(validate_note(&title, "").is_ok());
        assert!(validate_note(&"é".repeat(TITLE_MAX + 1), "").is_err());
    }

    #[test]
    fn an_empty_body_is_allowed() {
        assert!(validate_note("Title only", "").is_ok());
    }
}
