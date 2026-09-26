//! Data layer.
//!
//! Each file here owns one table: the Rust structs that mirror its schema and
//! the server functions that read and write it. The structs compile on every
//! target (the client needs them to render); the `#[get]`/`#[post]` bodies
//! only exist in the server build, and the client gets a generated stub that
//! makes the HTTP call.
//!
//! Adding a table means three things in step: a `.surql` file under
//! `database/schema`, a module here, and a `mod`/`pub use` pair below.

#[cfg(feature = "server")]
mod connection;
mod ids;
mod note;
mod user;
mod utils;

#[cfg(feature = "server")]
pub use connection::*;
pub use ids::*;
pub use note::*;
pub use user::*;
pub use utils::*;
