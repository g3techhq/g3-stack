//! The worked example.
//!
//! Four screens covering the shapes most features need: a list, a detail
//! page, a create form, and an edit form. Between them they show every
//! transition kind, `use_resource` fetching, optimistic mutation, and the
//! shared toast.
//!
//! Delete this directory (plus `src/db/note.rs` and
//! `database/schema/note.surql`) when you start on your own domain.

mod note_detail;
mod note_editor;
mod note_list;

pub use note_detail::*;
pub use note_editor::*;
pub use note_list::*;
