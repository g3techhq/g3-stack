//! Screens and shared UI.
//!
//! Layout: one directory per feature, plus `shell` for the parts that wrap
//! every screen. A directory's `mod.rs` re-exports its public components, and
//! this file re-exports the directories, so `app.rs` can import every screen
//! from one place and a feature can be deleted by removing one directory and
//! one line.

mod auth;
mod notes;
mod settings;
pub mod shared;
mod shell;

pub use auth::*;
pub use notes::*;
pub use settings::*;
pub use shell::*;
