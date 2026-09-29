use surrealdb_types::RecordId;

/// Anything that knows its own record id.
///
/// Useful for generic helpers over rows — deduplicating a list, keying a
/// `for` loop in `rsx!`, diffing a fetched page against what is on screen —
/// without each one needing its own concrete type.
///
/// `allow(dead_code)` rather than a call site invented to satisfy the lint:
/// this is API surface for your code, and the example screens happen to reach
/// for `record_key` directly. Delete the attribute the first time something
/// generic uses it.
#[allow(dead_code)]
pub trait Id {
    fn get_id(&self) -> RecordId;
}

macro_rules! impl_id {
    ($struct:ident) => {
        impl $crate::db::Id for $struct {
            fn get_id(&self) -> surrealdb_types::RecordId {
                self.id.clone()
            }
        }
    };
}

pub(crate) use impl_id;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Note, User};
    use surrealdb_types::Datetime;

    #[test]
    fn every_row_type_reports_its_own_id() {
        let user = User {
            id: RecordId::new("user", "u1"),
            handle: "someone".to_string(),
            display_name: "Someone".to_string(),
            email: None,
            appearance_mode: crate::db::AppearanceMode::Auto,
            color_scheme: crate::db::ColorScheme::Auto,
            created_at: Datetime::default(),
        };
        let note = Note {
            id: RecordId::new("note", "n1"),
            owner: user.id.clone(),
            title: "Title".to_string(),
            body: String::new(),
            pinned: false,
            created_at: Datetime::default(),
            updated_at: Datetime::default(),
        };

        // The point of the trait: one signature over both.
        fn key_of(row: &impl Id) -> String {
            crate::db::record_key(&row.get_id())
        }

        assert_eq!(key_of(&user), "u1");
        assert_eq!(key_of(&note), "n1");
    }
}
